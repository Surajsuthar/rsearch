use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Posting {
    pub doc_id: u32,
    pub term_freq: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocMeta {
    pub path: PathBuf, // empty PathBuf == tombstoned (removed) doc
    pub len: u32,      // token count
    pub mtime: u64,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Index {
    pub terms: HashMap<String, Vec<Posting>>,
    pub docs: Vec<DocMeta>,
    pub path_to_id: HashMap<PathBuf, u32>,
    pub total_tokens: u64,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn avg_doc_len(&self) -> f32 {
        let live = self
            .docs
            .iter()
            .filter(|d| !d.path.as_os_str().is_empty())
            .count();
        if live == 0 {
            0.0
        } else {
            self.total_tokens as f32 / live as f32
        }
    }

    pub fn live_doc_count(&self) -> usize {
        self.docs
            .iter()
            .filter(|d| !d.path.as_os_str().is_empty())
            .count()
    }

    /// Tombstone a doc if present: existing postings are left in place (and
    /// filtered out at query time) rather than rewritten.
    pub fn remove_doc(&mut self, path: &PathBuf) {
        if let Some(&doc_id) = self.path_to_id.get(path) {
            if let Some(meta) = self.docs.get_mut(doc_id as usize) {
                self.total_tokens = self.total_tokens.saturating_sub(meta.len as u64);
                meta.len = 0;
                meta.path = PathBuf::new();
            }
            self.path_to_id.remove(path);
        }
    }

    /// Index (or re-index) a document. If `path` was already indexed, the
    /// old version is tombstoned first and a fresh doc_id is assigned.
    pub fn add_doc(&mut self, path: PathBuf, tokens: &[String], mtime: u64, size: u64) {
        self.remove_doc(&path);

        let doc_id = self.docs.len() as u32;
        let mut tf: HashMap<&str, u32> = HashMap::new();
        for t in tokens {
            *tf.entry(t.as_str()).or_insert(0) += 1;
        }
        for (term, freq) in tf {
            self.terms
                .entry(term.to_string())
                .or_default()
                .push(Posting {
                    doc_id,
                    term_freq: freq,
                });
        }

        self.total_tokens += tokens.len() as u64;
        self.docs.push(DocMeta {
            path: path.clone(),
            len: tokens.len() as u32,
            mtime,
            size,
        });
        self.path_to_id.insert(path, doc_id);
    }
}
