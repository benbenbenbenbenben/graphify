use crate::detect::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_classify_python() {
    assert_eq!(classify_file(Path::new("foo.py")), Some(FileType::Code));
}

#[test]
fn test_classify_typescript() {
    assert_eq!(classify_file(Path::new("bar.ts")), Some(FileType::Code));
}

#[test]
fn test_classify_markdown() {
    assert_eq!(classify_file(Path::new("README.md")), Some(FileType::Document));
}

#[test]
fn test_classify_pdf() {
    assert_eq!(classify_file(Path::new("paper.pdf")), Some(FileType::Paper));
}

#[test]
fn test_classify_unknown_returns_none() {
    assert_eq!(classify_file(Path::new("archive.zip")), None);
}

#[test]
fn test_classify_image() {
    assert_eq!(classify_file(Path::new("screenshot.png")), Some(FileType::Image));
    assert_eq!(classify_file(Path::new("design.jpg")), Some(FileType::Image));
    assert_eq!(classify_file(Path::new("diagram.webp")), Some(FileType::Image));
}

#[test]
fn test_count_words_sample_md() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("sample.md");
    fs::write(&f, "This is a test document with more than five words").unwrap();
    let words = count_words(&f);
    assert!(words > 5);
}

#[test]
fn test_classify_md_paper_by_signals() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("paper.md");
    fs::write(&f, "# Abstract\n\nWe propose a new method. See [1] and [23].\nThis work was published in the Journal of AI. ArXiv preprint.\nSee Equation 3 for details. \\cite{vaswani2017}.\n").unwrap();
    assert_eq!(classify_file(&f), Some(FileType::Paper));
}

#[test]
fn test_classify_md_doc_without_signals() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("notes.md");
    fs::write(&f, "# My Notes\n\nHere are some notes about the project.\n").unwrap();
    assert_eq!(classify_file(&f), Some(FileType::Document));
}

#[test]
fn test_graphifyignore_excludes_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".graphifyignore"), "vendor/\n*.generated.py\n").unwrap();
    let vendor = dir.path().join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(vendor.join("lib.py"), "x = 1").unwrap();
    fs::write(dir.path().join("main.py"), "print('hi')").unwrap();
    fs::write(dir.path().join("schema.generated.py"), "x = 1").unwrap();

    let result = detect(dir.path(), false);
    let code_files = &result.files["code"];

    assert!(code_files.iter().any(|f| f.contains("main.py")));
    assert!(!code_files.iter().any(|f| f.contains("vendor")));
    assert!(!code_files.iter().any(|f| f.contains("generated")));
    assert_eq!(result.graphifyignore_patterns, 2);
}

#[test]
fn test_graphifyignore_comments_ignored() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".graphifyignore"), "# this is a comment\n\nmain.py\n").unwrap();
    fs::write(dir.path().join("main.py"), "x = 1").unwrap();
    fs::write(dir.path().join("other.py"), "x = 2").unwrap();

    let result = detect(dir.path(), false);
    let code_files = &result.files["code"];

    assert!(!code_files.iter().any(|f| f.contains("main.py")));
    assert!(code_files.iter().any(|f| f.contains("other.py")));
}
