use crate::validate::{validate_extraction, Extraction, ValidationError};
use petgraph::graph::{Graph, NodeIndex};
use std::collections::HashMap;

// We need a way to store arbitrary extra attributes for nodes and edges
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub file_type: String,
    pub source_file: String,
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub relation: String,
    pub confidence: String,
    pub source_file: String,
    pub extra: serde_json::Map<String, serde_json::Value>,
    pub original_source: String,
    pub original_target: String,
}

pub type MyGraph = Graph<GraphNode, GraphEdge, petgraph::Undirected>;

pub fn build_from_json(extraction: &Extraction) -> MyGraph {
    let errors = validate_extraction(extraction);
    let real_errors: Vec<_> = errors.into_iter()
        .filter(|e| !matches!(e, ValidationError::DanglingSource { .. } | ValidationError::DanglingTarget { .. }))
        .collect();

    if !real_errors.is_empty() {
        eprintln!("[graphify] Extraction warning ({} issues): {}", real_errors.len(), real_errors[0]);
    }

    let mut g = MyGraph::default();
    let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();

    for node in &extraction.nodes {
        let n = GraphNode {
            id: node.id.clone(),
            label: node.label.clone(),
            file_type: node.file_type.clone(),
            source_file: node.source_file.clone(),
            extra: node.extra.clone(),
        };
        // Node deduplication: just overwrite if already exists
        if let Some(&idx) = node_indices.get(&node.id) {
            *g.node_weight_mut(idx).unwrap() = n;
        } else {
            let idx = g.add_node(n);
            node_indices.insert(node.id.clone(), idx);
        }
    }

    for edge in &extraction.edges {
        if let (Some(&src_idx), Some(&tgt_idx)) = (node_indices.get(&edge.source), node_indices.get(&edge.target)) {
            let e = GraphEdge {
                relation: edge.relation.clone(),
                confidence: edge.confidence.clone(),
                source_file: edge.source_file.clone(),
                extra: edge.extra.clone(),
                original_source: edge.source.clone(),
                original_target: edge.target.clone(),
            };
            g.add_edge(src_idx, tgt_idx, e);
        }
    }

    g
}

pub fn build(extractions: &[Extraction]) -> MyGraph {
    let mut combined = Extraction::default();
    for ext in extractions {
        combined.nodes.extend(ext.nodes.clone());
        combined.edges.extend(ext.edges.clone());
        combined.hyperedges.extend(ext.hyperedges.clone());
        combined.input_tokens += ext.input_tokens;
        combined.output_tokens += ext.output_tokens;
    }
    build_from_json(&combined)
}
