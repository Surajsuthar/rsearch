use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct FoundFile {
    pub path: PathBuf,
    pub mtime: u64,
    pub size: u64,
}

/// Extensions we treat as text without needing to sniff the file.
/// Extend this list as needed — it's the cheapest way to widen scope.
const TEXTY_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "rs", "py", "js", "jsx", "ts", "tsx", "go", "c", "h",
    "cpp", "hpp", "cc", "java", "rb", "php", "html", "htm", "css", "scss", "json",
    "yaml", "yml", "toml", "csv", "tsv", "log", "sh", "bash", "sql", "xml", "ini",
    "cfg", "conf", "rst", "tex", "el", "lua", "kt", "swift", "scala", "clj", "hs",
    "wiki", "wikitext",
];

/// Walk `root`, respecting .gitignore-style rules, and return every file
/// that looks like text — either by extension or by content sniffing for
/// extension-less files (e.g. WikiExtractor output like `AA/wiki_00`).
pub fn discover(root: &Path) -> Vec<FoundFile> {
    let mut out = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(true) // skip dotfiles/dirs, including our own .rsearch/
        .git_ignore(true)
        .git_global(false)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.components().any(|c| c.as_os_str() == ".rsearch") {
            continue;
        }

        let looks_texty = match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => TEXTY_EXTENSIONS.contains(&ext.to_lowercase().as_str()),
            None => is_probably_text(path),
        };
        if !looks_texty {
            continue;
        }

        if let Ok(meta) = fs::metadata(path) {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push(FoundFile {
                path: path.to_path_buf(),
                mtime,
                size: meta.len(),
            });
        }
    }
    out
}

/// Heuristic for extension-less files: read the first 8KB, reject if it
/// contains a NUL byte or too high a share of non-printable bytes.
fn is_probably_text(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 8192];
    let Ok(n) = f.read(&mut buf) else {
        return false;
    };
    if n == 0 {
        return true;
    }
    let sample = &buf[..n];
    if sample.contains(&0) {
        return false;
    }
    let non_printable = sample.iter().filter(|&&b| b < 9 || (b > 13 && b < 32)).count();
    (non_printable as f32 / n as f32) < 0.05
}
