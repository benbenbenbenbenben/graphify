use crate::watch::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_notify_only_creates_flag() {
    let dir = TempDir::new().unwrap();
    notify_only(dir.path());
    let flag = dir.path().join("graphify-out").join("needs_update");
    assert!(flag.exists());
    assert_eq!(fs::read_to_string(&flag).unwrap(), "1");
}

#[test]
fn test_notify_only_creates_flag_dir() {
    let dir = TempDir::new().unwrap();
    assert!(!dir.path().join("graphify-out").exists());
    notify_only(dir.path());
    assert!(dir.path().join("graphify-out").is_dir());
}

#[test]
fn test_notify_only_idempotent() {
    let dir = TempDir::new().unwrap();
    notify_only(dir.path());
    notify_only(dir.path());
    let flag = dir.path().join("graphify-out").join("needs_update");
    assert_eq!(fs::read_to_string(&flag).unwrap(), "1");
}

#[test]
fn test_watched_extensions_includes_code() {
    assert!(WATCHED_EXTENSIONS.contains("py"));
    assert!(WATCHED_EXTENSIONS.contains("ts"));
    assert!(WATCHED_EXTENSIONS.contains("go"));
    assert!(WATCHED_EXTENSIONS.contains("rs"));
}

#[test]
fn test_watched_extensions_includes_docs() {
    assert!(WATCHED_EXTENSIONS.contains("md"));
    assert!(WATCHED_EXTENSIONS.contains("txt"));
    assert!(WATCHED_EXTENSIONS.contains("pdf"));
}

#[test]
fn test_watched_extensions_includes_images() {
    assert!(WATCHED_EXTENSIONS.contains("png"));
    assert!(WATCHED_EXTENSIONS.contains("jpg"));
}

#[test]
fn test_watched_extensions_excludes_noise() {
    assert!(!WATCHED_EXTENSIONS.contains("json"));
    assert!(!WATCHED_EXTENSIONS.contains("pyc"));
    assert!(!WATCHED_EXTENSIONS.contains("log"));
}
