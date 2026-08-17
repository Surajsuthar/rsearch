/// Lowercase, unicode-aware tokenizer. Splits on any non-alphanumeric
/// boundary. Kept deliberately simple for v1 — no stemming, no stopword
/// removal. Both are easy to bolt on later without touching the index
/// format (they only affect what strings get inserted as terms).
pub fn tokenize(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in lower.chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Truncate a string to at most `n` chars (unicode-safe), appending an
/// ellipsis if it was cut short.
pub fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let t: String = s.chars().take(n).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}
