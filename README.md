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
