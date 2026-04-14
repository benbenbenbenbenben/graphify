use crate::ingest::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_file_created() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let out = save_query_result("what is attention?", "Attention is...", &mem, "query", None).unwrap();
    assert!(out.exists());
}

#[test]
fn test_filename_format() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let out = save_query_result("what connects A to B?", "They share...", &mem, "query", None).unwrap();
    let name = out.file_name().unwrap().to_string_lossy();
    assert!(name.starts_with("query_"));
    assert!(name.ends_with(".md"));
}

#[test]
fn test_frontmatter_question() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let out = save_query_result("what is attention?", "Attention is softmax.", &mem, "query", None).unwrap();
    let content = fs::read_to_string(out).unwrap();
    assert!(content.contains("question:"));
    assert!(content.to_lowercase().contains("attention"));
}

#[test]
fn test_frontmatter_type() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let out = save_query_result("q", "a", &mem, "path_query", None).unwrap();
    let content = fs::read_to_string(out).unwrap();
    assert!(content.contains("type: \"path_query\""));
}

#[test]
fn test_source_nodes_included() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let nodes = vec!["AttentionLayer".to_string(), "SoftmaxFunc".to_string()];
    let out = save_query_result("q", "a", &mem, "query", Some(&nodes)).unwrap();
    let content = fs::read_to_string(out).unwrap();
    assert!(content.contains("AttentionLayer"));
    assert!(content.contains("SoftmaxFunc"));
}

#[test]
fn test_source_nodes_capped_at_10() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let mut nodes = Vec::new();
    for i in 0..20 { nodes.push(format!("Node{}", i)); }
    let out = save_query_result("q", "a", &mem, "query", Some(&nodes)).unwrap();
    let content = fs::read_to_string(out).unwrap();
    let fm_line = content.lines().find(|l| l.starts_with("source_nodes:")).unwrap();
    assert_eq!(fm_line.matches("\"Node").count(), 10);
}

#[test]
fn test_memory_dir_created() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("deep").join("memory");
    assert!(!mem.exists());
    save_query_result("q", "a", &mem, "query", None).unwrap();
    assert!(mem.exists());
}

#[test]
fn test_answer_in_body() {
    let dir = TempDir::new().unwrap();
    let mem = dir.path().join("memory");
    let out = save_query_result("what is the answer?", "The answer is forty-two.", &mem, "query", None).unwrap();
    let content = fs::read_to_string(out).unwrap();
    assert!(content.contains("The answer is forty-two."));
}
