use crate::validate::*;
use serde_json::json;

#[test]
fn test_valid_passes() {
    let extraction = Extraction {
        nodes: vec![
            Node {
                id: "n1".into(),
                label: "Foo".into(),
                file_type: "code".into(),
                source_file: "foo.py".into(),
                extra: Default::default(),
            },
            Node {
                id: "n2".into(),
                label: "Bar".into(),
                file_type: "document".into(),
                source_file: "bar.md".into(),
                extra: Default::default(),
            },
        ],
        edges: vec![
            Edge {
                source: "n1".into(),
                target: "n2".into(),
                relation: "references".into(),
                confidence: "EXTRACTED".into(),
                source_file: "foo.py".into(),
                extra: {
                    let mut map = serde_json::Map::new();
                    map.insert("weight".to_string(), json!(1.0));
                    map
                },
            },
        ],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}

#[test]
fn test_invalid_file_type() {
    let extraction = Extraction {
        nodes: vec![
            Node {
                id: "n1".into(),
                label: "X".into(),
                file_type: "video".into(),
                source_file: "x.mp4".into(),
                extra: Default::default(),
            }
        ],
        edges: vec![],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidFileType { .. })));
}

#[test]
fn test_invalid_confidence() {
    let extraction = Extraction {
        nodes: vec![
            Node { id: "n1".into(), label: "A".into(), file_type: "code".into(), source_file: "a.py".into(), extra: Default::default() },
            Node { id: "n2".into(), label: "B".into(), file_type: "code".into(), source_file: "b.py".into(), extra: Default::default() },
        ],
        edges: vec![
            Edge {
                source: "n1".into(),
                target: "n2".into(),
                relation: "calls".into(),
                confidence: "CERTAIN".into(),
                source_file: "a.py".into(),
                extra: Default::default(),
            }
        ],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidConfidence { .. })));
}

#[test]
fn test_dangling_edge_source() {
    let extraction = Extraction {
        nodes: vec![
            Node { id: "n1".into(), label: "A".into(), file_type: "code".into(), source_file: "a.py".into(), extra: Default::default() },
        ],
        edges: vec![
            Edge {
                source: "missing_id".into(),
                target: "n1".into(),
                relation: "calls".into(),
                confidence: "EXTRACTED".into(),
                source_file: "a.py".into(),
                extra: Default::default(),
            }
        ],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::DanglingSource { source_id, .. } if source_id == "missing_id")));
}

#[test]
fn test_dangling_edge_target() {
    let extraction = Extraction {
        nodes: vec![
            Node { id: "n1".into(), label: "A".into(), file_type: "code".into(), source_file: "a.py".into(), extra: Default::default() },
        ],
        edges: vec![
            Edge {
                source: "n1".into(),
                target: "ghost".into(),
                relation: "calls".into(),
                confidence: "EXTRACTED".into(),
                source_file: "a.py".into(),
                extra: Default::default(),
            }
        ],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::DanglingTarget { target, .. } if target == "ghost")));
}

#[test]
fn test_missing_node_field() {
    let extraction = Extraction {
        nodes: vec![
            Node {
                id: "n1".into(),
                label: "A".into(),
                file_type: "".into(), // Will be caught as invalid type, but let's test if it's correctly handled.
                source_file: "a.py".into(),
                extra: Default::default(),
            }
        ],
        edges: vec![],
        ..Default::default()
    };

    let errors = validate_extraction(&extraction);
    // Because Serde handles missing fields during deserialization, we don't have a specific test for missing file_type
    // here unless it's an empty string. The invalid file type check will catch the empty string.
    assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidFileType { .. })));
}
