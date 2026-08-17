use rsearch::{bm25, index::Index, storage};
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("pass a directory")).canonicalize().unwrap();

    // Pass 1: fresh build
    let (idx, stats) = storage::build_or_update(&root, Index::new(), |_, _| {}).unwrap();
    println!(
        "pass1: {} docs, added={} updated={} removed={}",
        stats.total_docs, stats.added, stats.updated, stats.removed
    );
    assert_eq!(stats.total_docs, 3, "expected 3 indexable docs (wiki_00, .md, .txt), .git should be skipped");
    storage::save(&root, &idx).unwrap();

    // Pass 2: load from disk, should be a no-op incremental update
    let loaded = storage::load(&root).expect("index should load");
    assert_eq!(loaded.live_doc_count(), 3);
    let (idx2, stats2) = storage::build_or_update(&root, loaded, |_, _| {}).unwrap();
    println!(
        "pass2 (no changes): added={} updated={} removed={}",
        stats2.added, stats2.updated, stats2.removed
    );
    assert_eq!((stats2.added, stats2.updated, stats2.removed), (0, 0, 0), "nothing changed, should be a no-op");

    // Search checks
    let r = bm25::search(&idx2, "inverted index postgresql", 10);
    println!("query 'inverted index postgresql' ->");
    for x in &r {
        println!("  {:.4}  {:?}", x.score, idx2.docs[x.doc_id as usize].path);
    }
    assert!(r[0].score > 0.0);
    assert!(idx2.docs[r[0].doc_id as usize].path.to_string_lossy().contains("postgres_internals"));

    let r2 = bm25::search(&idx2, "borrow checker memory safety", 10);
    println!("query 'borrow checker memory safety' ->");
    for x in &r2 {
        println!("  {:.4}  {:?}", x.score, idx2.docs[x.doc_id as usize].path);
    }
    assert!(idx2.docs[r2[0].doc_id as usize].path.to_string_lossy().contains("wiki_00"));

    // Modify a file, verify incremental update picks it up
    std::thread::sleep(std::time::Duration::from_secs(1));
    std::fs::write(root.join("notes.txt"), "shopping list now mentions quantum computing research").unwrap();
    let (idx3, stats3) = storage::build_or_update(&root, idx2, |_, _| {}).unwrap();
    println!("pass3 (notes.txt changed): added={} updated={} removed={}", stats3.added, stats3.updated, stats3.removed);
    assert_eq!((stats3.added, stats3.updated, stats3.removed), (0, 1, 0));
    let r3 = bm25::search(&idx3, "quantum computing", 10);
    assert_eq!(r3.len(), 1);
    assert!(idx3.docs[r3[0].doc_id as usize].path.to_string_lossy().contains("notes.txt"));

    println!("\nALL E2E CHECKS PASSED");
}
