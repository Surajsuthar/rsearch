use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::index::Index;
use crate::{tokenizer, walker};

const INDEX_DIR: &str = ".rsearch";
const INDEX_FILE: &str = "index.bin";

pub fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_DIR).join(INDEX_FILE)
}

pub fn load(root: &Path) -> Option<Index> {
    let bytes = fs::read(index_path(root)).ok()?;
    bincode::deserialize(&bytes).ok()
}

pub fn save(root: &Path, index: &Index) -> Result<()> {
    let dir = root.join(INDEX_DIR);
    fs::create_dir_all(&dir)?;
    let bytes = bincode::serialize(index)?;
    fs::write(index_path(root), bytes)?;
    Ok(())
}

pub struct BuildStats {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub total_docs: usize,
    pub elapsed_ms: u128,
}

/// Build (or incrementally refresh) the index for `root`. Files are
/// re-tokenized only if their mtime or size changed since last indexed;
/// files that vanished from disk get tombstoned.
pub fn build_or_update(
    root: &Path,
    mut existing: Index,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<(Index, BuildStats)> {
    let start = Instant::now();
    let found = walker::discover(root);
    let total = found.len();

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut added = 0usize;
    let mut updated = 0usize;

    for (i, f) in found.iter().enumerate() {
        seen.insert(f.path.clone());

        let needs_index = match existing.path_to_id.get(&f.path) {
            Some(&doc_id) => {
                let meta = &existing.docs[doc_id as usize];
                meta.mtime != f.mtime || meta.size != f.size
            }
            None => true,
        };

        if needs_index {
            if let Ok(content) = fs::read_to_string(&f.path) {
                let tokens = tokenizer::tokenize(&content);
                let was_new = !existing.path_to_id.contains_key(&f.path);
                existing.add_doc(f.path.clone(), &tokens, f.mtime, f.size);
                if was_new {
                    added += 1;
                } else {
                    updated += 1;
                }
            }
            // Files that fail UTF-8 decoding are silently skipped for now —
            // a binary-safe/lossy path is an easy follow-up if needed.
        }
        on_progress(i + 1, total);
    }

    let stale: Vec<PathBuf> = existing
        .path_to_id
        .keys()
        .filter(|p| !seen.contains(*p))
        .cloned()
        .collect();
    let removed = stale.len();
    for p in stale {
        existing.remove_doc(&p);
    }

    let stats = BuildStats {
        added,
        updated,
        removed,
        total_docs: existing.live_doc_count(),
        elapsed_ms: start.elapsed().as_millis(),
    };

    Ok((existing, stats))
}
