use crate::security::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_validate_url_accepts_http() {
    assert_eq!(validate_url("http://example.com/page").unwrap(), "http://example.com/page");
}

#[test]
fn test_validate_url_accepts_https() {
    assert_eq!(validate_url("https://arxiv.org/abs/1706.03762").unwrap(), "https://arxiv.org/abs/1706.03762");
}

#[test]
fn test_validate_url_rejects_file() {
    assert!(matches!(validate_url("file:///etc/passwd"), Err(SecurityError::InvalidScheme(_))));
}

#[test]
fn test_validate_url_rejects_ftp() {
    assert!(matches!(validate_url("ftp://files.example.com/data.zip"), Err(SecurityError::InvalidScheme(_))));
}

#[test]
fn test_validate_url_rejects_data() {
    assert!(matches!(validate_url("data:text/html,<script>alert(1)</script>"), Err(SecurityError::InvalidScheme(_))));
}

#[test]
fn test_validate_url_rejects_empty_scheme() {
    assert!(validate_url("//no-scheme.example.com").is_err());
}

#[test]
fn test_safe_fetch_rejects_file_url() {
    assert!(matches!(safe_fetch("file:///etc/passwd", MAX_FETCH_BYTES), Err(SecurityError::InvalidScheme(_))));
}

#[test]
fn test_safe_fetch_rejects_ftp_url() {
    assert!(matches!(safe_fetch("ftp://example.com/file.zip", MAX_FETCH_BYTES), Err(SecurityError::InvalidScheme(_))));
}

#[test]
fn test_validate_graph_path_allows_inside_base() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join("graphify-out");
    fs::create_dir(&base).unwrap();
    let graph = base.join("graph.json");
    fs::write(&graph, "{}").unwrap();

    let result = validate_graph_path(&graph, Some(&base)).unwrap();
    assert_eq!(result, std::fs::canonicalize(graph).unwrap());
}

#[test]
fn test_validate_graph_path_blocks_traversal() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join("graphify-out");
    fs::create_dir(&base).unwrap();
    let evil = base.join("..").join("etc_passwd");

    assert!(matches!(validate_graph_path(&evil, Some(&base)), Err(SecurityError::PathEscapesBase)));
}

#[test]
fn test_validate_graph_path_requires_base_exists() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join("graphify-out"); // not created
    let graph = base.join("graph.json");

    assert!(matches!(validate_graph_path(&graph, Some(&base)), Err(SecurityError::BaseDoesNotExist(_))));
}

#[test]
fn test_validate_graph_path_raises_if_file_missing() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join("graphify-out");
    fs::create_dir(&base).unwrap();
    let missing = base.join("missing.json");

    assert!(matches!(validate_graph_path(&missing, Some(&base)), Err(SecurityError::FileNotFound(_))));
}

#[test]
fn test_sanitize_label_passthrough_html_chars() {
    assert_eq!(sanitize_label("<script>"), "<script>");
    assert_eq!(sanitize_label("foo & bar"), "foo & bar");
}

#[test]
fn test_sanitize_label_strips_control_chars() {
    let result = sanitize_label("hello\x00\x1fworld");
    assert!(!result.contains('\x00'));
    assert!(!result.contains('\x1f'));
    assert!(result.contains("helloworld"));
}

#[test]
fn test_sanitize_label_caps_at_256() {
    let long_label = "a".repeat(300);
    assert_eq!(sanitize_label(&long_label).len(), 256);
}

#[test]
fn test_sanitize_label_safe_passthrough() {
    assert_eq!(sanitize_label("MyClass"), "MyClass");
    assert_eq!(sanitize_label("extract_python"), "extract_python");
}
