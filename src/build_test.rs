use crate::build::*;
use crate::validate::{Extraction, Node, Edge};

fn load_extraction() -> Extraction {
    let content = std::fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn test_build_from_json_node_count() {
    let g = build_from_json(&load_extraction());
    assert_eq!(g.node_count(), 4);
}

#[test]
fn test_build_from_json_edge_count() {
    let g = build_from_json(&load_extraction());
    assert_eq!(g.edge_count(), 4);
}

#[test]
fn test_nodes_have_label() {
    let g = build_from_json(&load_extraction());
    let mut found = false;
    for node in g.node_weights() {
        if node.id == "n_transformer" {
            assert_eq!(node.label, "Transformer");
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn test_edges_have_confidence() {
    let g = build_from_json(&load_extraction());
    let mut found = false;
    for edge in g.edge_weights() {
        if edge.original_source == "n_attention" && edge.original_target == "n_concept_attn" {
            assert_eq!(edge.confidence, "INFERRED");
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn test_ambiguous_edge_preserved() {
    let g = build_from_json(&load_extraction());
    let mut found = false;
    for edge in g.edge_weights() {
        if edge.original_source == "n_layernorm" && edge.original_target == "n_concept_attn" {
            assert_eq!(edge.confidence, "AMBIGUOUS");
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn test_build_merges_multiple_extractions() {
    let ext1 = Extraction {
        nodes: vec![
            Node { id: "n1".into(), label: "A".into(), file_type: "code".into(), source_file: "a.py".into(), extra: Default::default() },
        ],
        edges: vec![],
        ..Default::default()
    };
    let ext2 = Extraction {
        nodes: vec![
            Node { id: "n2".into(), label: "B".into(), file_type: "document".into(), source_file: "b.md".into(), extra: Default::default() },
        ],
        edges: vec![
            Edge {
                source: "n1".into(),
                target: "n2".into(),
                relation: "references".into(),
                confidence: "INFERRED".into(),
                source_file: "b.md".into(),
                extra: Default::default(),
            }
        ],
        ..Default::default()
    };
    let graph = build(&[ext1, ext2]);
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}
