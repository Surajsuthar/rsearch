use crate::bm25;
use crate::index::Index;
use crate::tokenizer;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
}

pub struct ResultRow {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

pub struct App {
    pub index: Index,
    pub mode: Mode,
    pub query: String,
    pub results: Vec<ResultRow>,
    pub selected: usize,
    pub status: String,
}

impl App {
    pub fn new(index: Index) -> Self {
        Self {
            index,
            mode: Mode::Normal,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            status: String::new(),
        }
    }

    pub fn run_search(&mut self) {
        let scored = bm25::search(&self.index, &self.query, 200);
        let query = self.query.clone();
        self.results = scored
            .into_iter()
            .filter_map(|s| {
                let meta = self.index.docs.get(s.doc_id as usize)?;
                if meta.path.as_os_str().is_empty() {
                    return None;
                }
                Some(ResultRow {
                    path: meta.path.display().to_string(),
                    score: s.score,
                    snippet: snippet_for(&meta.path, &query),
                })
            })
            .collect();
        self.selected = 0;
        self.status = format!("{} results for \"{}\"", self.results.len(), self.query);
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() {
            return;
        }
        let len = self.results.len() as i32;
        let mut new = self.selected as i32 + delta;
        new = new.clamp(0, len - 1);
        self.selected = new as usize;
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.results.get(self.selected).map(|r| r.path.as_str())
    }
}

/// Grab the first line containing a query term, for a quick preview under
/// each result. Re-reads the file rather than storing positions in the
/// index — simpler, and fine at the file sizes this targets; if you later
/// index single giant files (e.g. a raw Wikipedia XML dump) this is the
/// spot to swap in stored token positions instead.
fn snippet_for(path: &std::path::Path, query: &str) -> String {
    let terms = tokenizer::tokenize(query);
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    for line in content.lines() {
        let lower = line.to_lowercase();
        if terms.iter().any(|t| lower.contains(t.as_str())) {
            return tokenizer::truncate_chars(line.trim(), 140);
        }
    }
    String::new()
}
