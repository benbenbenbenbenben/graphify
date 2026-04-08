use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Code,
    Document,
    Paper,
    Image,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Code => "code",
            FileType::Document => "document",
            FileType::Paper => "paper",
            FileType::Image => "image",
        }
    }
}

pub const CORPUS_WARN_THRESHOLD: usize = 50_000;
pub const CORPUS_UPPER_THRESHOLD: usize = 500_000;
pub const FILE_COUNT_UPPER: usize = 200;
pub const MANIFEST_PATH: &str = "graphify-out/manifest.json";

lazy_static::lazy_static! {
    static ref SENSITIVE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(^|[\\/])\.(env|envrc)(\.|$)").unwrap(),
        Regex::new(r"(?i)\.(pem|key|p12|pfx|cert|crt|der|p8)$").unwrap(),
        Regex::new(r"(?i)(credential|secret|passwd|password|token|private_key)").unwrap(),
        Regex::new(r"(?i)(id_rsa|id_dsa|id_ecdsa|id_ed25519)(\.pub)?$").unwrap(),
        Regex::new(r"(?i)(\.netrc|\.pgpass|\.htpasswd)$").unwrap(),
        Regex::new(r"(?i)(aws_credentials|gcloud_credentials|service\.account)").unwrap(),
    ];

    static ref PAPER_SIGNALS: Vec<Regex> = vec![
        Regex::new(r"(?i)\barxiv\b").unwrap(),
        Regex::new(r"(?i)\bdoi\s*:").unwrap(),
        Regex::new(r"(?i)\babstract\b").unwrap(),
        Regex::new(r"(?i)\bproceedings\b").unwrap(),
        Regex::new(r"(?i)\bjournal\b").unwrap(),
        Regex::new(r"(?i)\bpreprint\b").unwrap(),
        Regex::new(r"\\cite\{").unwrap(),
        Regex::new(r"\[\d+\]").unwrap(),
        Regex::new(r"\[\n\d+\n\]").unwrap(),
        Regex::new(r"(?i)eq\.\s*\d+|equation\s+\d+").unwrap(),
        Regex::new(r"\d{4}\.\d{4,5}").unwrap(),
        Regex::new(r"(?i)\bwe propose\b").unwrap(),
        Regex::new(r"(?i)\bliterature\b").unwrap(),
    ];

    static ref CODE_EXTENSIONS: HashSet<&'static str> = vec![
        "py", "ts", "js", "tsx", "go", "rs", "java", "cpp", "cc", "cxx", "c", "h", "hpp",
        "rb", "swift", "kt", "kts", "cs", "scala", "php", "lua", "toc", "zig", "ps1", "ex", "exs", "m", "mm"
    ].into_iter().collect();

    static ref DOC_EXTENSIONS: HashSet<&'static str> = vec!["md", "txt", "rst"].into_iter().collect();
    static ref PAPER_EXTENSIONS: HashSet<&'static str> = vec!["pdf"].into_iter().collect();
    static ref IMAGE_EXTENSIONS: HashSet<&'static str> = vec!["png", "jpg", "jpeg", "gif", "webp", "svg"].into_iter().collect();
    static ref OFFICE_EXTENSIONS: HashSet<&'static str> = vec!["docx", "xlsx"].into_iter().collect();

    static ref SKIP_DIRS: HashSet<&'static str> = vec![
        "venv", ".venv", "env", ".env", "node_modules", "__pycache__", ".git",
        "dist", "build", "target", "out", "site-packages", "lib64",
        ".pytest_cache", ".mypy_cache", ".ruff_cache", ".tox", ".eggs"
    ].into_iter().collect();
}

pub fn is_sensitive(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let full = path.to_string_lossy();
    SENSITIVE_PATTERNS.iter().any(|p| p.is_match(&name) || p.is_match(&full))
}

pub fn looks_like_paper(path: &Path) -> bool {
    if let Ok(text) = fs::read_to_string(path) {
        let snippet = if text.len() > 3000 { &text[..3000] } else { &text };
        let hits = PAPER_SIGNALS.iter().filter(|p| p.is_match(snippet)).count();
        hits >= 3
    } else {
        false
    }
}

pub fn classify_file(path: &Path) -> Option<FileType> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let ext_str = ext.as_str();

    if CODE_EXTENSIONS.contains(ext_str) {
        return Some(FileType::Code);
    }
    if PAPER_EXTENSIONS.contains(ext_str) {
        return Some(FileType::Paper);
    }
    if IMAGE_EXTENSIONS.contains(ext_str) {
        return Some(FileType::Image);
    }
    if DOC_EXTENSIONS.contains(ext_str) {
        if looks_like_paper(path) {
            return Some(FileType::Paper);
        }
        return Some(FileType::Document);
    }
    if OFFICE_EXTENSIONS.contains(ext_str) {
        return Some(FileType::Document);
    }
    None
}

pub fn count_words(path: &Path) -> usize {
    if let Ok(text) = fs::read_to_string(path) {
        text.split_whitespace().count()
    } else {
        0
    }
}

fn is_noise_dir(part: &str) -> bool {
    if SKIP_DIRS.contains(part) {
        return true;
    }
    if part.ends_with("_venv") || part.ends_with("_env") || part.ends_with(".egg-info") {
        return true;
    }
    false
}

fn load_graphifyignore(root: &Path) -> Vec<String> {
    let ignore_file = root.join(".graphifyignore");
    let mut patterns = Vec::new();
    if let Ok(content) = fs::read_to_string(ignore_file) {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                patterns.push(line.to_string());
            }
        }
    }
    patterns
}

fn is_ignored(path: &Path, root: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    if let Ok(rel) = path.strip_prefix(root) {
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let parts: Vec<&str> = rel_str.split('/').collect();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        for pattern in patterns {
            let p = pattern.trim_matches('/');
            if p.is_empty() { continue; }

            if let Ok(pat) = glob::Pattern::new(p) {
                if pat.matches(&rel_str) || pat.matches(&file_name) {
                    return true;
                }
                for i in 0..parts.len() {
                    if pat.matches(parts[i]) || pat.matches(&parts[..=i].join("/")) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DetectResult {
    pub files: HashMap<String, Vec<String>>,
    pub total_files: usize,
    pub total_words: usize,
    pub needs_graph: bool,
    pub warning: Option<String>,
    pub skipped_sensitive: Vec<String>,
    pub graphifyignore_patterns: usize,
}

pub fn detect(root: &Path, follow_symlinks: bool) -> DetectResult {
    let mut files = HashMap::new();
    files.insert("code".to_string(), Vec::new());
    files.insert("document".to_string(), Vec::new());
    files.insert("paper".to_string(), Vec::new());
    files.insert("image".to_string(), Vec::new());

    let mut total_words = 0;
    let mut skipped_sensitive = Vec::new();
    let ignore_patterns = load_graphifyignore(root);

    let memory_dir = root.join("graphify-out").join("memory");
    let mut scan_paths = vec![root.to_path_buf()];
    if memory_dir.exists() {
        scan_paths.push(memory_dir.clone());
    }

    let mut seen = HashSet::new();
    let mut all_files = Vec::new();

    for scan_root in scan_paths {
        let in_memory_tree = memory_dir.exists() && scan_root.starts_with(&memory_dir);
        let mut walker = WalkDir::new(&scan_root).follow_links(follow_symlinks);

        let it = walker.into_iter();
        for entry in it.filter_map(|e| e.ok()) {
            let p = entry.path();
            if entry.file_type().is_dir() {
                if !in_memory_tree {
                    let name = entry.file_name().to_string_lossy();
                    if name.starts_with('.') || is_noise_dir(&name) || is_ignored(p, root, &ignore_patterns) {
                        // In a real implementation we'd prune the traversal here.
                        // WalkDir does this via filter_entry, but for simplicity we skip.
                    }
                }
            } else if entry.file_type().is_file() || entry.file_type().is_symlink() {
                let mut valid = true;
                if !in_memory_tree {
                    for ancestor in p.ancestors() {
                        if ancestor == root { break; }
                        let name = ancestor.file_name().unwrap_or_default().to_string_lossy();
                        if name.starts_with('.') || is_noise_dir(&name) || is_ignored(ancestor, root, &ignore_patterns) {
                            valid = false;
                            break;
                        }
                    }
                }

                if valid {
                    if let Ok(canon) = std::fs::canonicalize(p) {
                        if seen.insert(canon) {
                            all_files.push(p.to_path_buf());
                        }
                    } else if seen.insert(p.to_path_buf()) {
                        all_files.push(p.to_path_buf());
                    }
                }
            }
        }
    }

    let converted_dir = root.join("graphify-out").join("converted");

    for p in all_files {
        let in_memory = memory_dir.exists() && p.starts_with(&memory_dir);
        if !in_memory {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.') { continue; }
            if p.starts_with(&converted_dir) { continue; }
        }
        if is_ignored(&p, root, &ignore_patterns) { continue; }
        if is_sensitive(&p) {
            skipped_sensitive.push(p.to_string_lossy().into_owned());
            continue;
        }

        if let Some(ftype) = classify_file(&p) {
            files.get_mut(ftype.as_str()).unwrap().push(p.to_string_lossy().into_owned());
            total_words += count_words(&p);
        }
    }

    let total_files: usize = files.values().map(|v| v.len()).sum();
    let needs_graph = total_words >= CORPUS_WARN_THRESHOLD;

    let warning = if !needs_graph {
        Some(format!("Corpus is ~{} words - fits in a single context window. You may not need a graph.", total_words))
    } else if total_words >= CORPUS_UPPER_THRESHOLD || total_files >= FILE_COUNT_UPPER {
        Some(format!("Large corpus: {} files · ~{} words. Semantic extraction will be expensive (many Claude tokens). Consider running on a subfolder, or use --no-semantic to run AST-only.", total_files, total_words))
    } else {
        None
    };

    DetectResult {
        files,
        total_files,
        total_words,
        needs_graph,
        warning,
        skipped_sensitive,
        graphifyignore_patterns: ignore_patterns.len(),
    }
}

pub fn load_manifest(manifest_path: &str) -> HashMap<String, f64> {
    if let Ok(content) = fs::read_to_string(manifest_path) {
        if let Ok(m) = serde_json::from_str(&content) {
            return m;
        }
    }
    HashMap::new()
}

pub fn save_manifest(files: &HashMap<String, Vec<String>>, manifest_path: &str) {
    let mut manifest: HashMap<String, f64> = HashMap::new();
    for file_list in files.values() {
        for f in file_list {
            if let Ok(metadata) = fs::metadata(f) {
                if let Ok(mtime) = metadata.modified() {
                    let epoch = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
                    manifest.insert(f.clone(), epoch);
                }
            }
        }
    }
    if let Some(parent) = Path::new(manifest_path).parent() {
        fs::create_dir_all(parent).unwrap_or_default();
    }
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(manifest_path, json);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IncrementalDetectResult {
    #[serde(flatten)]
    pub detect: DetectResult,
    pub incremental: bool,
    pub new_files: HashMap<String, Vec<String>>,
    pub unchanged_files: HashMap<String, Vec<String>>,
    pub new_total: usize,
    pub deleted_files: Vec<String>,
}

pub fn detect_incremental(root: &Path, manifest_path: &str) -> IncrementalDetectResult {
    let full = detect(root, false);
    let manifest = load_manifest(manifest_path);

    if manifest.is_empty() {
        return IncrementalDetectResult {
            incremental: true,
            new_files: full.files.clone(),
            unchanged_files: [("code", vec![]), ("document", vec![]), ("paper", vec![]), ("image", vec![])].into_iter().map(|(k,v)| (k.to_string(), v)).collect(),
            new_total: full.total_files,
            deleted_files: vec![],
            detect: full,
        };
    }

    let mut new_files = HashMap::new();
    let mut unchanged_files = HashMap::new();
    for key in full.files.keys() {
        new_files.insert(key.clone(), Vec::new());
        unchanged_files.insert(key.clone(), Vec::new());
    }

    let mut current_files = HashSet::new();

    for (ftype, file_list) in &full.files {
        for f in file_list {
            current_files.insert(f.clone());
            let current_mtime = fs::metadata(f)
                .and_then(|m| m.modified())
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64())
                .unwrap_or(0.0);

            let stored_mtime = manifest.get(f).copied();

            if stored_mtime.is_none() || current_mtime > stored_mtime.unwrap() {
                new_files.get_mut(ftype).unwrap().push(f.clone());
            } else {
                unchanged_files.get_mut(ftype).unwrap().push(f.clone());
            }
        }
    }

    let deleted_files: Vec<String> = manifest.keys().filter(|f| !current_files.contains(*f)).cloned().collect();
    let new_total = new_files.values().map(|v| v.len()).sum();

    IncrementalDetectResult {
        incremental: true,
        new_files,
        unchanged_files,
        new_total,
        deleted_files,
        detect: full,
    }
}
