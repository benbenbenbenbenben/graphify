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

/// Validate a raw `serde_json::Value`, matching the Python API `validate_extraction(data: dict)`.
///
/// Unlike `validate_extraction`, this function detects missing required fields and a
/// non-object top-level value, producing `MissingNodeField` / `MissingEdgeField` /
/// `NotAnObject` errors that would otherwise surface as serde parse failures.
pub fn validate_extraction_value(value: &serde_json::Value) -> Vec<ValidationError> {
    use serde_json::Value;

    let obj = match value.as_object() {
        Some(o) => o,
        None => return vec![ValidationError::NotAnObject],
    };

    let mut errors = Vec::new();
    let valid_file_types: HashSet<&str> = ["code", "document", "paper", "image", "rationale"].into_iter().collect();
    let valid_confidences: HashSet<&str> = ["EXTRACTED", "INFERRED", "AMBIGUOUS"].into_iter().collect();

    const NODE_REQUIRED: &[&str] = &["id", "label", "file_type", "source_file"];
    const EDGE_REQUIRED: &[&str] = &["source", "target", "relation", "confidence", "source_file"];

    let mut node_ids: HashSet<String> = HashSet::new();

    let nodes = obj.get("nodes").and_then(Value::as_array).map(|a| a.as_slice()).unwrap_or(&[]);
    for (i, node) in nodes.iter().enumerate() {
        let id = node.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        for &field in NODE_REQUIRED {
            if node.get(field).map_or(true, |v| v.is_null()) {
                errors.push(ValidationError::MissingNodeField { index: i, id: id.clone(), field: field.to_string() });
            }
        }
        if !id.is_empty() {
            node_ids.insert(id.clone());
        }
        if let Some(ft) = node.get("file_type").and_then(Value::as_str) {
            if !valid_file_types.contains(ft) {
                errors.push(ValidationError::InvalidFileType { index: i, id: id.clone(), file_type: ft.to_string() });
            }
        }
    }

    let edges = obj.get("edges").and_then(Value::as_array).map(|a| a.as_slice()).unwrap_or(&[]);
    for (i, edge) in edges.iter().enumerate() {
        for &field in EDGE_REQUIRED {
            if edge.get(field).map_or(true, |v| v.is_null()) {
                errors.push(ValidationError::MissingEdgeField { index: i, field: field.to_string() });
            }
        }
        if let Some(conf) = edge.get("confidence").and_then(Value::as_str) {
            if !valid_confidences.contains(conf) {
                errors.push(ValidationError::InvalidConfidence { index: i, confidence: conf.to_string() });
            }
        }
        if !node_ids.is_empty() {
            if let Some(src) = edge.get("source").and_then(Value::as_str) {
                if !node_ids.contains(src) {
                    errors.push(ValidationError::DanglingSource { index: i, source_id: src.to_string() });
                }
            }
            if let Some(tgt) = edge.get("target").and_then(Value::as_str) {
                if !node_ids.contains(tgt) {
                    errors.push(ValidationError::DanglingTarget { index: i, target: tgt.to_string() });
                }
            }
        }
    }

    errors
}
