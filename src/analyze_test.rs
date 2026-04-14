use crate::analyze::*;
use crate::build::*;
use crate::cluster::*;
use crate::validate::{Extraction, Node, Edge};
use std::fs;

fn make_graph() -> MyGraph {
    let content = fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    let ext: Extraction = serde_json::from_str(&content).unwrap();
    build_from_json(&ext)
}

#[test]
fn test_god_nodes_returns_list() {
    let g = make_graph();
    let result = god_nodes(&g, 3);
    assert!(result.len() <= 3);
}

#[test]
fn test_god_nodes_sorted_by_degree() {
    let g = make_graph();
    let result = god_nodes(&g, 10);
    let degrees: Vec<usize> = result.iter().map(|r| r.edges).collect();
    let mut sorted_degrees = degrees.clone();
    sorted_degrees.sort_by(|a, b| b.cmp(a));
    assert_eq!(degrees, sorted_degrees);
}

#[test]
fn test_god_nodes_have_required_keys() {
    let g = make_graph();
    let result = god_nodes(&g, 1);
    if let Some(first) = result.first() {
        assert!(!first.id.is_empty());
        assert!(!first.label.is_empty());
    }
}

#[test]
fn test_surprising_connections_cross_source_multi_file() {
    let g = make_graph();
    let communities = cluster(&g);
    let surprises = surprising_connections(&g, Some(&communities), 5);
    assert!(!surprises.is_empty());
    for s in surprises {
        assert_ne!(s.source_files[0], s.source_files[1]);
    }
}

#[test]
fn test_surprising_connections_excludes_concept_nodes() {
    let mut ext: Extraction = serde_json::from_str(&fs::read_to_string("tests/fixtures/extraction.json").unwrap()).unwrap();
    ext.nodes.push(Node { id: "concept_x".into(), label: "Abstract Concept".into(), file_type: "document".into(), source_file: "".into(), extra: Default::default() });
    ext.edges.push(Edge { source: "n_transformer".into(), target: "concept_x".into(), relation: "relates_to".into(), confidence: "INFERRED".into(), source_file: "".into(), extra: Default::default() });
    let g = build_from_json(&ext);
    let communities = cluster(&g);
    let surprises = surprising_connections(&g, Some(&communities), 5);
    for s in surprises {
        assert_ne!(s.source, "Abstract Concept");
        assert_ne!(s.target, "Abstract Concept");
    }
}

fn make_simple_graph(nodes: &[(&str, &str)], edges: &[(&str, &str, &str, &str)]) -> MyGraph {
    let mut g = MyGraph::default();
    let mut id_to_idx = std::collections::HashMap::new();
    for &(id, label) in nodes {
        let idx = g.add_node(GraphNode { id: id.into(), label: label.into(), file_type: "".into(), source_file: "test.py".into(), extra: Default::default() });
        id_to_idx.insert(id, idx);
    }
    for &(src, tgt, rel, conf) in edges {
        g.add_edge(id_to_idx[src], id_to_idx[tgt], GraphEdge { relation: rel.into(), confidence: conf.into(), source_file: "test.py".into(), original_source: src.into(), original_target: tgt.into(), extra: Default::default() });
    }
    g
}

#[test]
fn test_graph_diff_new_nodes() {
    let g_old = make_simple_graph(&[("n1", "Alpha"), ("n2", "Beta")], &[]);
    let g_new = make_simple_graph(&[("n1", "Alpha"), ("n2", "Beta"), ("n3", "Gamma")], &[]);
    let diff = graph_diff(&g_old, &g_new);
    assert_eq!(diff.new_nodes.len(), 1);
    assert_eq!(diff.new_nodes[0].id, "n3");
    assert_eq!(diff.new_nodes[0].label, "Gamma");
    assert!(diff.removed_nodes.is_empty());
    assert!(diff.summary.contains("1 new node"));
}

#[test]
fn test_graph_diff_removed_nodes() {
    let g_old = make_simple_graph(&[("n1", "Alpha"), ("n2", "Beta"), ("n3", "Gamma")], &[]);
    let g_new = make_simple_graph(&[("n1", "Alpha"), ("n2", "Beta")], &[]);
    let diff = graph_diff(&g_old, &g_new);
    assert!(diff.new_nodes.is_empty());
    assert_eq!(diff.removed_nodes.len(), 1);
    assert_eq!(diff.removed_nodes[0].id, "n3");
    assert!(diff.summary.contains("removed"));
}

#[test]
fn test_graph_diff_new_edges() {
    let nodes = vec![("n1", "Alpha"), ("n2", "Beta"), ("n3", "Gamma")];
    let g_old = make_simple_graph(&nodes, &[("n1", "n2", "calls", "EXTRACTED")]);
    let g_new = make_simple_graph(&nodes, &[("n1", "n2", "calls", "EXTRACTED"), ("n2", "n3", "uses", "INFERRED")]);
    let diff = graph_diff(&g_old, &g_new);
    assert_eq!(diff.new_edges.len(), 1);
    assert_eq!(diff.new_edges[0].relation, "uses");
    assert_eq!(diff.new_edges[0].confidence, "INFERRED");
    assert!(diff.removed_edges.is_empty());
    assert!(diff.summary.contains("new edge"));
}

#[test]
fn test_graph_diff_empty_diff() {
    let nodes = vec![("n1", "Alpha"), ("n2", "Beta")];
    let edges = vec![("n1", "n2", "calls", "EXTRACTED")];
    let g_old = make_simple_graph(&nodes, &edges);
    let g_new = make_simple_graph(&nodes, &edges);
    let diff = graph_diff(&g_old, &g_new);
    assert!(diff.new_nodes.is_empty());
    assert!(diff.removed_nodes.is_empty());
    assert!(diff.new_edges.is_empty());
    assert!(diff.removed_edges.is_empty());
    assert_eq!(diff.summary, "no changes");
}
