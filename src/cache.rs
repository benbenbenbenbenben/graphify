use crate::validate::{Extraction, Node, Edge};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn file_hash(path: &Path) -> std::io::Result<String> {
    let content = fs::read(path)?;
    let resolved = std::fs::canonicalize(path)?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    hasher.update(b"\x00");
    hasher.update(resolved.to_string_lossy().as_bytes());

    Ok(hex::encode(hasher.finalize()))
}

pub fn cache_dir(root: &Path) -> PathBuf {
    let d = root.join("graphify-out").join("cache");
    fs::create_dir_all(&d).unwrap_or_default();
    d
}

pub fn load_cached(path: &Path, root: &Path) -> Option<Extraction> {
    let h = file_hash(path).ok()?;
    let entry = cache_dir(root).join(format!("{}.json", h));

    if !entry.exists() {
        return None;
    }

    let content = fs::read_to_string(&entry).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_cached(path: &Path, result: &Extraction, root: &Path) -> std::io::Result<()> {
    let h = file_hash(path)?;
    let entry = cache_dir(root).join(format!("{}.json", h));
    let mut tmp = entry.clone();
    tmp.set_extension("tmp");

    let json_str = serde_json::to_string(result)?;
    if let Err(e) = fs::write(&tmp, json_str) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    if let Err(e) = fs::rename(&tmp, &entry) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    Ok(())
}

pub fn cached_files(root: &Path) -> HashSet<String> {
    let mut hashes = HashSet::new();
    let d = cache_dir(root);

    if let Ok(entries) = fs::read_dir(d) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    hashes.insert(stem.to_string());
                }
            }
        }
    }

    hashes
}

pub fn clear_cache(root: &Path) {
    let d = cache_dir(root);
    if let Ok(entries) = fs::read_dir(d) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub fn check_semantic_cache(files: &[String], root: &Path) -> (Vec<Node>, Vec<Edge>, Vec<serde_json::Value>, Vec<String>) {
    let mut cached_nodes = Vec::new();
    let mut cached_edges = Vec::new();
    let mut cached_hyperedges = Vec::new();
    let mut uncached = Vec::new();

    for fpath in files {
        if let Some(result) = load_cached(Path::new(fpath), root) {
            cached_nodes.extend(result.nodes);
            cached_edges.extend(result.edges);
            cached_hyperedges.extend(result.hyperedges);
        } else {
            uncached.push(fpath.clone());
        }
    }

    (cached_nodes, cached_edges, cached_hyperedges, uncached)
}

pub fn save_semantic_cache(
    nodes: &[Node],
    edges: &[Edge],
    hyperedges: Option<&[serde_json::Value]>,
    root: &Path,
) -> usize {
    let mut by_file: HashMap<String, Extraction> = HashMap::new();

    for n in nodes {
        if !n.source_file.is_empty() {
            let ext = by_file.entry(n.source_file.clone()).or_default();
            ext.nodes.push(n.clone());
        }
    }

    for e in edges {
        if !e.source_file.is_empty() {
            let ext = by_file.entry(e.source_file.clone()).or_default();
            ext.edges.push(e.clone());
        }
    }

    if let Some(hyperedges_slice) = hyperedges {
        for h in hyperedges_slice {
            if let Some(src) = h.get("source_file").and_then(|s| s.as_str()) {
                if !src.is_empty() {
                    let ext = by_file.entry(src.to_string()).or_default();
                    ext.hyperedges.push(h.clone());
                }
            }
        }
    }

    let mut saved = 0;
    for (fpath, result) in by_file {
        let mut p = PathBuf::from(&fpath);
        if !p.is_absolute() {
            p = root.join(&p);
        }
        if p.exists() {
            if save_cached(&p, &result, root).is_ok() {
                saved += 1;
            }
        }
    }

    saved
}
