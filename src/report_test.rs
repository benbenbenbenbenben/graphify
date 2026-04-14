use crate::report::*;
use crate::build::*;
use crate::cluster::*;
use crate::analyze::*;
use crate::detect::*;
use crate::validate::Extraction;
use std::collections::HashMap;
use std::fs;

fn make_inputs() -> (MyGraph, HashMap<usize, Vec<String>>, HashMap<usize, f64>, HashMap<usize, String>, Vec<GodNode>, Vec<Surprise>, DetectResult, (usize, usize)) {
    let content = fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    let extraction: Extraction = serde_json::from_str(&content).unwrap();
    let g = build_from_json(&extraction);
    let communities = cluster(&g);
    let cohesion = score_all(&g, &communities);
    let mut labels = HashMap::new();
    for cid in communities.keys() { labels.insert(*cid, format!("Community {}", cid)); }
    let gods = god_nodes(&g, 10);
    let surprises = surprising_connections(&g, Some(&communities), 5);
    let mut detection = DetectResult::default();
    detection.total_files = 4; detection.total_words = 62400; detection.needs_graph = true;
    let tokens = (extraction.input_tokens as usize, extraction.output_tokens as usize);
    (g, communities, cohesion, labels, gods, surprises, detection, tokens)
}

#[test]
fn test_report_contains_header() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("# Graph Report"));
}

#[test]
fn test_report_contains_corpus_check() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("## Corpus Check"));
}

#[test]
fn test_report_contains_god_nodes() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("## God Nodes"));
}

#[test]
fn test_report_contains_surprising_connections() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("## Surprising Connections"));
}

#[test]
fn test_report_contains_communities() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("## Communities"));
}

#[test]
fn test_report_contains_ambiguous_section() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("## Ambiguous Edges"));
}

#[test]
fn test_report_shows_token_cost() {
    let (g, comms, coh, labels, gods, surps, det, mut toks) = make_inputs();
    toks.0 = 1200;
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("Token cost"));
    assert!(report.contains("1200"));
}

#[test]
fn test_report_shows_raw_cohesion_scores() {
    let (g, comms, coh, labels, gods, surps, det, toks) = make_inputs();
    let report = generate(&g, &comms, &coh, &labels, &gods, &surps, &det, &toks, "./project", None);
    assert!(report.contains("Cohesion:"));
    assert!(!report.contains('✓'));
    assert!(!report.contains('⚠'));
}
