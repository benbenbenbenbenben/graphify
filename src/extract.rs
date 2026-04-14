use crate::validate::Extraction;
use std::path::Path;

pub fn collect_files(root: &Path, follow_symlinks: bool) -> Vec<String> {
    let detect_result = crate::detect::detect(root, follow_symlinks);
    let mut files = Vec::new();
    if let Some(code) = detect_result.files.get("code") {
        files.extend(code.clone());
    }
    files
}

pub fn extract(_files: &[String]) -> Extraction {
    let ext = Extraction::default();
    ext
}
