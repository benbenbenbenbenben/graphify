use crate::cache::*;
use crate::validate::{Extraction, Node, Edge};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_file_hash_consistent() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("sample.txt");
    fs::write(&f, "hello world").unwrap();

    let h1 = file_hash(&f).unwrap();
    let h2 = file_hash(&f).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn test_file_hash_changes() {
    let dir = TempDir::new().unwrap();
    let f1 = dir.path().join("a.txt");
    let f2 = dir.path().join("b.txt");
    fs::write(&f1, "content one").unwrap();
    fs::write(&f2, "content two").unwrap();

    assert_ne!(file_hash(&f1).unwrap(), file_hash(&f2).unwrap());
}

#[test]
fn test_cache_roundtrip() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let f = root.join("sample.txt");
    fs::write(&f, "hello world").unwrap();

    let result = Extraction {
        nodes: vec![
            Node { id: "n1".into(), label: "Node1".into(), file_type: "code".into(), source_file: "sample.txt".into(), extra: Default::default() }
        ],
        ..Default::default()
    };

    save_cached(&f, &result, root).unwrap();
    let loaded = load_cached(&f, root).unwrap();
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].id, "n1");
}

#[test]
fn test_cache_miss_on_change() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let f = root.join("sample.txt");
    fs::write(&f, "hello world").unwrap();

    let result = Extraction {
        edges: vec![
            Edge { source: "a".into(), target: "b".into(), relation: "c".into(), confidence: "EXTRACTED".into(), source_file: "sample.txt".into(), extra: Default::default() }
        ],
        ..Default::default()
    };

    save_cached(&f, &result, root).unwrap();
    fs::write(&f, "completely different content").unwrap();
    assert!(load_cached(&f, root).is_none());
}

#[test]
fn test_cached_files() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let f1 = root.join("file1.py");
    let f2 = root.join("file2.py");
    fs::write(&f1, "alpha").unwrap();
    fs::write(&f2, "beta").unwrap();

    save_cached(&f1, &Extraction::default(), root).unwrap();
    save_cached(&f2, &Extraction::default(), root).unwrap();

    let hashes = cached_files(root);
    assert!(hashes.contains(&file_hash(&f1).unwrap()));
    assert!(hashes.contains(&file_hash(&f2).unwrap()));
}

#[test]
fn test_clear_cache() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let f = root.join("sample.txt");
    fs::write(&f, "hello world").unwrap();

    save_cached(&f, &Extraction::default(), root).unwrap();
    let cache_d = root.join("graphify-out").join("cache");
    assert!(fs::read_dir(&cache_d).unwrap().count() > 0);

    clear_cache(root);
    assert_eq!(fs::read_dir(&cache_d).unwrap().count(), 0);
}
