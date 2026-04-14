use crate::cluster::*;
use crate::build::*;
use crate::validate::Extraction;
use petgraph::graph::Graph;
use std::collections::HashSet;
use std::fs;

fn make_graph() -> MyGraph {
    let content = fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    let ext: Extraction = serde_json::from_str(&content).unwrap();
    build_from_json(&ext)
}

#[test]
fn test_cluster_returns_dict() {
    let g = make_graph();
    let communities = cluster(&g);
    assert!(!communities.is_empty());
}

#[test]
fn test_cluster_covers_all_nodes() {
    let g = make_graph();
    let communities = cluster(&g);
    let mut all_nodes = HashSet::new();
    for nodes in communities.values() { for n in nodes { all_nodes.insert(n.clone()); } }
    let mut g_nodes = HashSet::new();
    for node in g.node_weights() { g_nodes.insert(node.id.clone()); }
    assert_eq!(all_nodes, g_nodes);
}

#[test]
fn test_cohesion_score_complete_graph() {
    let mut g = MyGraph::default();
    let n0 = g.add_node(GraphNode { id: "0".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    let n1 = g.add_node(GraphNode { id: "1".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    let n2 = g.add_node(GraphNode { id: "2".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });

    g.add_edge(n0, n1, GraphEdge { relation: "".into(), confidence: "".into(), source_file: "".into(), extra: Default::default(), original_source: "0".into(), original_target: "1".into() });
    g.add_edge(n1, n2, GraphEdge { relation: "".into(), confidence: "".into(), source_file: "".into(), extra: Default::default(), original_source: "1".into(), original_target: "2".into() });
    g.add_edge(n0, n2, GraphEdge { relation: "".into(), confidence: "".into(), source_file: "".into(), extra: Default::default(), original_source: "0".into(), original_target: "2".into() });

    let score = cohesion_score(&g, &["0".into(), "1".into(), "2".into()]);
    assert_eq!(score, 1.0);
}

#[test]
fn test_cohesion_score_single_node() {
    let mut g = MyGraph::default();
    g.add_node(GraphNode { id: "a".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    let score = cohesion_score(&g, &["a".into()]);
    assert_eq!(score, 1.0);
}

#[test]
fn test_cohesion_score_disconnected() {
    let mut g = MyGraph::default();
    g.add_node(GraphNode { id: "a".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    g.add_node(GraphNode { id: "b".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    g.add_node(GraphNode { id: "c".into(), label: "".into(), file_type: "".into(), source_file: "".into(), extra: Default::default() });
    let score = cohesion_score(&g, &["a".into(), "b".into(), "c".into()]);
    assert_eq!(score, 0.0);
}

#[test]
fn test_cohesion_score_range() {
    let g = make_graph();
    let communities = cluster(&g);
    for nodes in communities.values() {
        let score = cohesion_score(&g, nodes);
        assert!(score >= 0.0 && score <= 1.0);
    }
}

#[test]
fn test_score_all_keys_match_communities() {
    let g = make_graph();
    let communities = cluster(&g);
    let scores = score_all(&g, &communities);
    assert_eq!(scores.len(), communities.len());
    for key in communities.keys() { assert!(scores.contains_key(key)); }
}
