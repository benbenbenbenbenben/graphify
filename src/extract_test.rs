use crate::extract::*;
use std::path::Path;

#[test]
fn test_collect_files_empty() {
    let files = collect_files(Path::new("non_existent"), false);
    assert!(files.is_empty());
}

#[test]
fn test_extract_empty() {
    let ext = extract(&[]);
    assert!(ext.nodes.is_empty());
    assert!(ext.edges.is_empty());
}
