# rsearch

Local full-text search over a directory tree. Point it at a folder, it
builds a BM25-ranked inverted index, and drops you into a vim-style TUI to
search it.

## Build & run

```
cargo build --release
cd /path/to/some/directory   # e.g. your Wikipedia dump / extracted text
/path/to/rsearch/target/release/rsearch
```

Or run against a specific directory without `cd`-ing:

```
rsearch /path/to/wikipedia-extracted/
```

First run indexes everything and writes `.rsearch/index.bin` inside the
target directory. Subsequent runs load that index and only re-tokenize
files whose mtime/size changed — so relaunching against a huge corpus
(Wikipedia-scale) is fast after the first pass.

Flags:
- `--rebuild` / `-r` — ignore the existing index and reindex from scratch.

## TUI keys

- `/` — enter search mode, type your query
- `Enter` (in search mode) — run the search, ranked by BM25
- `j` / `k` or arrows — move selection through results
- `Enter` (in normal mode) — open the selected file in `$EDITOR` (falls back to `vi`)
- `r` — reindex now (incremental)
- `q` / `Esc` — quit

## How it works

- **Discovery** (`walker.rs`): walks the tree via the `ignore` crate
  (respects `.gitignore`, skips hidden dirs). Files are included by
  extension, or by content-sniffing (reject on NUL byte / high
  non-printable ratio) for extension-less files — covers things like
  WikiExtractor output (`AA/wiki_00`, no extension).
- **Indexing** (`index.rs`): a term → postings inverted index
  (`doc_id`, `term_freq` per posting). Deletions/re-indexes are handled by
  tombstoning rather than in-place postings rewrites — same idea as an
  LSM tree deferring cleanup to a compaction pass. There's no `compact()`
  yet; add one if you're reindexing a fast-churning tree a lot.
- **Ranking** (`bm25.rs`): standard Okapi BM25 (k1=1.2, b=0.75), summed
  across query terms rather than a strict boolean AND, so partial matches
  still surface (just ranked lower).
- **Persistence** (`storage.rs`): the whole `Index` is `bincode`-serialized
  to `.rsearch/index.bin`. Simple and fast to implement; the honest
  trade-off is it's loaded fully into memory on startup rather than
  memory-mapped, so it won't scale gracefully to a truly enormous single
  index the way an on-disk B-tree/SSTable-segment layout would.
- **TUI** (`ui.rs`, `main.rs`): `ratatui` + `crossterm`, manual selection
  highlighting (no `ListState`) to keep the render path simple.

## Known limitations / natural next steps

- No phrase queries — this is a bag-of-words AND-of-scores model. Adding
  positions to `Posting` (currently just `term_freq`) is the way in.
- No stemming or stopword removal — easy to add in `tokenizer.rs` without
  touching the index format.
- One file = one document. If you want to index a single giant XML dump
  (raw Wikipedia dump rather than pre-extracted files), you'd add a
  splitter that chunks it into pseudo-documents before tokenizing.
- Tombstones accumulate on repeated reindex of a changing tree; there's no
  compaction pass yet.
- The on-disk format is a single serialized blob, not a disk-native
  structure — a good next step if you want this to double as a storage
  engine exercise (segment files + merge, or reuse the B+tree work for
  postings storage instead of a `HashMap`).

## Verification

This was actually compiled and exercised in a sandbox (not just written and
hoped for):

- `cargo build` passes clean (ratatui 0.28.1 / crossterm 0.28.1) — including
  the TUI event loop, `Frame::area()`, alternate-screen setup/teardown, and
  the `$EDITOR` suspend/resume path.
- The indexing/BM25/tombstoning logic was run through an isolated
  dependency-free smoke test asserting: correct ranking for distinct
  queries, empty results for a non-matching query, that re-indexing a file
  in place tombstones its old content (a stale term no longer matches),
  and that removing a doc drops it from future results.
- `src/bin/e2e_test.rs` (a throwaway test binary, delete it or leave it —
  doesn't affect `rsearch` itself) ran the *real* pipeline end-to-end
  against a sample directory: walking + `.gitignore`-respecting exclusion
  (a `.git/` dir was correctly skipped), extension-less file detection
  (mimicking WikiExtractor's `AA/wiki_00` naming), a save → load →
  reindex round trip that correctly no-ops when nothing changed, and a
  live file edit that triggered exactly one incremental re-index (not a
  full rebuild) with the new content immediately searchable.

What's *not* verified: interactive keyboard-driven TUI behavior itself
(the sandbox has no real tty to drive `crossterm`'s raw-mode input), so
give the `/`, `j/k`, `Enter`, `r` bindings a try and let me know if
anything feels off.
