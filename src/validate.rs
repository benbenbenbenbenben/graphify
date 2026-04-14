use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub file_type: String,
    pub source_file: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: String,
    pub source_file: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Extraction {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub hyperedges: Vec<serde_json::Value>,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}

#[derive(Error, Debug, PartialEq)]
pub enum ValidationError {
    #[error("Extraction must be a JSON object")]
    NotAnObject,
    #[error("Node {index} (id={id:?}) missing required field '{field}'")]
    MissingNodeField { index: usize, id: String, field: String },
    #[error("Node {index} (id={id:?}) has invalid file_type '{file_type}' - must be one of [\"code\", \"document\", \"image\", \"paper\", \"rationale\"]")]
    InvalidFileType { index: usize, id: String, file_type: String },
    #[error("Edge {index} missing required field '{field}'")]
    MissingEdgeField { index: usize, field: String },
    #[error("Edge {index} has invalid confidence '{confidence}' - must be one of [\"AMBIGUOUS\", \"EXTRACTED\", \"INFERRED\"]")]
    InvalidConfidence { index: usize, confidence: String },
    #[error("Edge {index} source '{source_id}' does not match any node id")]
    DanglingSource { index: usize, source_id: String },
    #[error("Edge {index} target '{target}' does not match any node id")]
    DanglingTarget { index: usize, target: String },
}

pub fn validate_extraction(extraction: &Extraction) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let valid_file_types: HashSet<&str> = ["code", "document", "paper", "image", "rationale"].into_iter().collect();
    let valid_confidences: HashSet<&str> = ["EXTRACTED", "INFERRED", "AMBIGUOUS"].into_iter().collect();

    let mut node_ids = HashSet::new();

    for (i, node) in extraction.nodes.iter().enumerate() {
        node_ids.insert(node.id.clone());
        if !valid_file_types.contains(node.file_type.as_str()) {
            errors.push(ValidationError::InvalidFileType {
                index: i,
                id: node.id.clone(),
                file_type: node.file_type.clone(),
            });
        }
    }

    for (i, edge) in extraction.edges.iter().enumerate() {
        if !valid_confidences.contains(edge.confidence.as_str()) {
            errors.push(ValidationError::InvalidConfidence {
                index: i,
                confidence: edge.confidence.clone(),
            });
        }

        if !node_ids.is_empty() {
            if !node_ids.contains(&edge.source) {
                errors.push(ValidationError::DanglingSource {
                    index: i,
                    source_id: edge.source.clone(),
                });
            }
            if !node_ids.contains(&edge.target) {
                errors.push(ValidationError::DanglingTarget {
                    index: i,
                    target: edge.target.clone(),
                });
            }
        }
    }

    errors
}
