use crate::export::*;
use crate::build::*;
use crate::cluster::*;
use crate::validate::Extraction;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn make_graph() -> MyGraph {
    let content = fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    let extraction: Extraction = serde_json::from_str(&content).unwrap();
    build_from_json(&extraction)
}

#[test]
fn test_to_json_creates_file() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.json");
    to_json(&g, &communities, out.to_str().unwrap());
    assert!(out.exists());
}

#[test]
fn test_to_json_valid_json() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.json");
    to_json(&g, &communities, out.to_str().unwrap());
    let content = fs::read_to_string(&out).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(data.get("nodes").is_some());
    assert!(data.get("links").is_some());
}

#[test]
fn test_to_cypher_creates_file() {
    let g = make_graph();
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("cypher.txt");
    to_cypher(&g, out.to_str().unwrap());
    assert!(out.exists());
}

#[test]
fn test_to_cypher_contains_merge_statements() {
    let g = make_graph();
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("cypher.txt");
    to_cypher(&g, out.to_str().unwrap());
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("MERGE"));
}

#[test]
fn test_to_graphml_creates_file() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.graphml");
    to_graphml(&g, &communities, out.to_str().unwrap());
    assert!(out.exists());
}

#[test]
fn test_to_graphml_valid_xml() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.graphml");
    to_graphml(&g, &communities, out.to_str().unwrap());
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("<graphml"));
    assert!(content.contains("<node"));
}

#[test]
fn test_to_html_creates_file() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.html");
    to_html(&g, &communities, out.to_str().unwrap(), None).unwrap();
    assert!(out.exists());
}

#[test]
fn test_to_html_contains_visjs() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.html");
    to_html(&g, &communities, out.to_str().unwrap(), None).unwrap();
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("vis-network"));
}

#[test]
fn test_to_html_contains_search() {
    let g = make_graph();
    let communities = cluster(&g);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("graph.html");
    to_html(&g, &communities, out.to_str().unwrap(), None).unwrap();
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.to_lowercase().contains("search"));
}
