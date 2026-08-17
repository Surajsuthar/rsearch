use crate::index::Index;
use crate::tokenizer;
use std::collections::HashMap;

const K1: f32 = 1.2;
const B: f32 = 0.75;

pub struct ScoredDoc {
    pub doc_id: u32,
    pub score: f32,
}

/// Rank documents against `query` using Okapi BM25, summed across query
/// terms (AND-of-scores, not a hard AND filter — docs missing some terms
/// still rank, just lower, which tends to feel better for exploratory
/// search than a strict boolean AND).
pub fn search(index: &Index, query: &str, limit: usize) -> Vec<ScoredDoc> {
    let terms = tokenizer::tokenize(query);
    if terms.is_empty() {
        return vec![];
    }

    let n = index.live_doc_count() as f32;
    if n == 0.0 {
        return vec![];
    }
    let avgdl = index.avg_doc_len().max(1.0);

    let mut scores: HashMap<u32, f32> = HashMap::new();

    for term in &terms {
        let Some(postings) = index.terms.get(term) else {
            continue;
        };

        let df = postings
            .iter()
            .filter(|p| {
                index
                    .docs
                    .get(p.doc_id as usize)
                    .map(|d| !d.path.as_os_str().is_empty())
                    .unwrap_or(false)
            })
            .count() as f32;
        if df == 0.0 {
            continue;
        }
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

        for p in postings {
            let Some(meta) = index.docs.get(p.doc_id as usize) else {
                continue;
            };
            if meta.path.as_os_str().is_empty() {
                continue; // tombstoned
            }
            let dl = meta.len.max(1) as f32;
            let tf = p.term_freq as f32;
            let denom = tf + K1 * (1.0 - B + B * dl / avgdl);
            let s = idf * (tf * (K1 + 1.0)) / denom;
            *scores.entry(p.doc_id).or_insert(0.0) += s;
        }
    }

    let mut result: Vec<ScoredDoc> = scores
        .into_iter()
        .map(|(doc_id, score)| ScoredDoc { doc_id, score })
        .collect();
    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    result.truncate(limit);
    result
}
