use crate::build::{MyGraph, GraphNode};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

fn is_concept_node(node: &GraphNode) -> bool {
    if node.source_file.is_empty() { return true; }
    let parts: Vec<&str> = node.source_file.split('/').collect();
    if let Some(last) = parts.last() {
        if !last.contains('.') { return true; }
    }
    false
}

fn is_file_node(g: &MyGraph, node_idx: petgraph::graph::NodeIndex) -> bool {
    let node = g.node_weight(node_idx).unwrap();
    if node.label.is_empty() { return false; }
    if !node.source_file.is_empty() {
        let path = std::path::Path::new(&node.source_file);
        if let Some(file_name) = path.file_name() {
            if node.label == file_name.to_string_lossy() { return true; }
        }
    }
    if node.label.starts_with('.') && node.label.ends_with("()") { return true; }
    if node.label.ends_with("()") && g.edges(node_idx).count() <= 1 { return true; }
    false
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GodNode { pub id: String, pub label: String, pub edges: usize }

pub fn god_nodes(g: &MyGraph, top_n: usize) -> Vec<GodNode> {
    let mut degrees: Vec<_> = g.node_indices().map(|idx| (idx, g.edges(idx).count())).collect();
    degrees.sort_by(|a, b| b.1.cmp(&a.1));
    let mut result = Vec::new();
    for (idx, deg) in degrees {
        let node = g.node_weight(idx).unwrap();
        if is_file_node(g, idx) || is_concept_node(node) { continue; }
        result.push(GodNode {
            id: node.id.clone(),
            label: if node.label.is_empty() { node.id.clone() } else { node.label.clone() },
            edges: deg,
        });
        if result.len() >= top_n { break; }
    }
    result
}

fn file_category(path: &str) -> &'static str {
    if path.is_empty() { return "doc"; }
    let ext = match path.rsplit_once('.') { Some((_, e)) => e.to_lowercase(), None => "".to_string() };
    match ext.as_str() {
        "py" | "ts" | "tsx" | "js" | "go" | "rs" | "java" | "rb" | "cpp" | "c" | "h" | "cs" | "kt" | "scala" | "php" => "code",
        "pdf" => "paper",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => "image",
        _ => "doc",
    }
}

fn top_level_dir(path: &str) -> &str { path.split('/').next().unwrap_or(path) }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Surprise {
    pub source: String, pub target: String, pub source_files: Vec<String>,
    pub confidence: String, pub relation: String, pub why: Option<String>,
    pub note: Option<String>,
    #[serde(skip)] pub _score: usize,
}

pub fn surprising_connections(g: &MyGraph, communities: Option<&HashMap<usize, Vec<String>>>, top_n: usize) -> Vec<Surprise> {
    let mut source_files = HashSet::new();
    for node in g.node_weights() {
        if !node.source_file.is_empty() { source_files.insert(node.source_file.clone()); }
    }
    let empty_map = HashMap::new();
    let comms = communities.unwrap_or(&empty_map);
    if source_files.len() > 1 { cross_file_surprises(g, comms, top_n) } else { cross_community_surprises(g, comms, top_n) }
}

fn cross_file_surprises(g: &MyGraph, communities: &HashMap<usize, Vec<String>>, top_n: usize) -> Vec<Surprise> {
    let mut node_community = HashMap::new();
    for (cid, nodes) in communities { for n in nodes { node_community.insert(n.clone(), *cid); } }
    let mut candidates = Vec::new();
    for edge in g.edge_references() {
        let u_idx = edge.source(); let v_idx = edge.target();
        let u_node = g.node_weight(u_idx).unwrap(); let v_node = g.node_weight(v_idx).unwrap();
        let relation = &edge.weight().relation;
        if ["imports", "imports_from", "contains", "method"].contains(&relation.as_str()) { continue; }
        if is_concept_node(u_node) || is_concept_node(v_node) { continue; }
        if is_file_node(g, u_idx) || is_file_node(g, v_idx) { continue; }
        let u_source = &u_node.source_file; let v_source = &v_node.source_file;
        if u_source.is_empty() || v_source.is_empty() || u_source == v_source { continue; }
        let (score, reasons) = surprise_score(g, u_idx, v_idx, edge.weight(), &node_community);
        let src_id = if edge.weight().original_source == u_node.id { &u_node.id } else { &v_node.id };
        let tgt_id = if edge.weight().original_target == v_node.id { &v_node.id } else { &u_node.id };
        candidates.push(Surprise {
            _score: score,
            source: if src_id == &u_node.id { u_node.label.clone() } else { v_node.label.clone() },
            target: if tgt_id == &u_node.id { u_node.label.clone() } else { v_node.label.clone() },
            source_files: vec![u_source.clone(), v_source.clone()],
            confidence: edge.weight().confidence.clone(),
            relation: relation.clone(),
            why: Some(if reasons.is_empty() { "cross-file semantic connection".into() } else { reasons.join("; ") }),
            note: None,
        });
    }
    candidates.sort_by(|a, b| b._score.cmp(&a._score));
    if !candidates.is_empty() { candidates.into_iter().take(top_n).collect() } else { cross_community_surprises(g, communities, top_n) }
}

fn cross_community_surprises(g: &MyGraph, communities: &HashMap<usize, Vec<String>>, top_n: usize) -> Vec<Surprise> {
    if communities.is_empty() { return Vec::new(); }
    let mut node_community = HashMap::new();
    for (cid, nodes) in communities { for n in nodes { node_community.insert(n.clone(), *cid); } }
    let mut surprises = Vec::new();
    for edge in g.edge_references() {
        let u_idx = edge.source(); let v_idx = edge.target();
        let u_node = g.node_weight(u_idx).unwrap(); let v_node = g.node_weight(v_idx).unwrap();
        let cid_u = node_community.get(&u_node.id); let cid_v = node_community.get(&v_node.id);
        if cid_u.is_none() || cid_v.is_none() || cid_u == cid_v { continue; }
        if is_file_node(g, u_idx) || is_file_node(g, v_idx) { continue; }
        let relation = &edge.weight().relation;
        if ["imports", "imports_from", "contains", "method"].contains(&relation.as_str()) { continue; }
        let confidence = edge.weight().confidence.clone();
        surprises.push((Surprise {
            _score: 0, source: u_node.label.clone(), target: v_node.label.clone(),
            source_files: vec![u_node.source_file.clone(), v_node.source_file.clone()],
            confidence: confidence.clone(), relation: relation.clone(),
            why: None, note: Some(format!("Bridges community {} → community {}", cid_u.unwrap(), cid_v.unwrap())),
        }, { let mut pair = vec![*cid_u.unwrap(), *cid_v.unwrap()]; pair.sort(); pair }));
    }
    surprises.sort_by(|a, b| {
        let order = |c: &str| match c { "AMBIGUOUS" => 0, "INFERRED" => 1, "EXTRACTED" => 2, _ => 3 };
        order(&a.0.confidence).cmp(&order(&b.0.confidence))
    });
    let mut deduped = Vec::new(); let mut seen = HashSet::new();
    for (s, pair) in surprises { if seen.insert(pair) { deduped.push(s); } }
    deduped.into_iter().take(top_n).collect()
}

fn surprise_score(
    g: &MyGraph, u_idx: petgraph::graph::NodeIndex, v_idx: petgraph::graph::NodeIndex,
    data: &crate::build::GraphEdge, node_community: &HashMap<String, usize>,
) -> (usize, Vec<String>) {
    let mut score = 0; let mut reasons = Vec::new();
    let u_node = g.node_weight(u_idx).unwrap(); let v_node = g.node_weight(v_idx).unwrap();
    let conf = data.confidence.as_str();
    score += match conf { "AMBIGUOUS" => 3, "INFERRED" => 2, "EXTRACTED" => 1, _ => 1 };
    if conf == "AMBIGUOUS" || conf == "INFERRED" { reasons.push(format!("{} connection - not explicitly stated in source", conf.to_lowercase())); }
    let cat_u = file_category(&u_node.source_file); let cat_v = file_category(&v_node.source_file);
    if cat_u != cat_v { score += 2; reasons.push(format!("crosses file types ({} ↔ {})", cat_u, cat_v)); }
    if top_level_dir(&u_node.source_file) != top_level_dir(&v_node.source_file) { score += 2; reasons.push("connects across different repos/directories".into()); }
    if let (Some(cid_u), Some(cid_v)) = (node_community.get(&u_node.id), node_community.get(&v_node.id)) {
        if cid_u != cid_v { score += 1; reasons.push("bridges separate communities".into()); }
    }
    if data.relation == "semantically_similar_to" { score = (score as f64 * 1.5) as usize; reasons.push("semantically similar concepts with no structural link".into()); }
    let deg_u = g.edges(u_idx).count(); let deg_v = g.edges(v_idx).count();
    if deg_u.min(deg_v) <= 2 && deg_u.max(deg_v) >= 5 {
        score += 1;
        let peripheral = if deg_u <= 2 { &u_node.label } else { &v_node.label };
        let hub = if deg_u <= 2 { &v_node.label } else { &u_node.label };
        reasons.push(format!("peripheral node `{}` unexpectedly reaches hub `{}`", peripheral, hub));
    }
    (score, reasons)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphDiff {
    pub new_nodes: Vec<DiffNode>, pub removed_nodes: Vec<DiffNode>,
    pub new_edges: Vec<DiffEdge>, pub removed_edges: Vec<DiffEdge>, pub summary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiffNode { pub id: String, pub label: String }

#[derive(Serialize, Deserialize, Debug)]
pub struct DiffEdge { pub source: String, pub target: String, pub relation: String, pub confidence: String }

pub fn graph_diff(g_old: &MyGraph, g_new: &MyGraph) -> GraphDiff {
    let old_nodes: HashMap<_, _> = g_old.node_weights().map(|n| (n.id.clone(), n.label.clone())).collect();
    let new_nodes: HashMap<_, _> = g_new.node_weights().map(|n| (n.id.clone(), n.label.clone())).collect();
    let mut added_nodes = Vec::new();
    for (id, label) in &new_nodes { if !old_nodes.contains_key(id) { added_nodes.push(DiffNode { id: id.clone(), label: label.clone() }); } }
    let mut removed_nodes = Vec::new();
    for (id, label) in &old_nodes { if !new_nodes.contains_key(id) { removed_nodes.push(DiffNode { id: id.clone(), label: label.clone() }); } }
    fn edge_key(src: &str, tgt: &str, rel: &str) -> String { let (u, v) = if src < tgt { (src, tgt) } else { (tgt, src) }; format!("{}:{}:{}", u, v, rel) }
    let mut old_edges = HashMap::new();
    for edge in g_old.edge_references() {
        let u_id = &g_old.node_weight(edge.source()).unwrap().id; let v_id = &g_old.node_weight(edge.target()).unwrap().id;
        old_edges.insert(edge_key(u_id, v_id, &edge.weight().relation), edge.weight());
    }
    let mut new_edges = HashMap::new();
    for edge in g_new.edge_references() {
        let u_id = &g_new.node_weight(edge.source()).unwrap().id; let v_id = &g_new.node_weight(edge.target()).unwrap().id;
        new_edges.insert(edge_key(u_id, v_id, &edge.weight().relation), edge);
    }
    let mut added_edges_list = Vec::new();
    for (key, edge) in &new_edges {
        if !old_edges.contains_key(key) {
            added_edges_list.push(DiffEdge {
                source: g_new.node_weight(edge.source()).unwrap().id.clone(),
                target: g_new.node_weight(edge.target()).unwrap().id.clone(),
                relation: edge.weight().relation.clone(), confidence: edge.weight().confidence.clone(),
            });
        }
    }
    let mut removed_edges_list = Vec::new();
    for (key, edge) in &old_edges {
        if !new_edges.contains_key(key) {
            removed_edges_list.push(DiffEdge {
                source: edge.original_source.clone(), target: edge.original_target.clone(),
                relation: edge.relation.clone(), confidence: edge.confidence.clone(),
            });
        }
    }
    let mut parts = Vec::new();
    if !added_nodes.is_empty() { parts.push(format!("{} new node{}", added_nodes.len(), if added_nodes.len() == 1 { "" } else { "s" })); }
    if !added_edges_list.is_empty() { parts.push(format!("{} new edge{}", added_edges_list.len(), if added_edges_list.len() == 1 { "" } else { "s" })); }
    if !removed_nodes.is_empty() { parts.push(format!("{} node{} removed", removed_nodes.len(), if removed_nodes.len() == 1 { "" } else { "s" })); }
    if !removed_edges_list.is_empty() { parts.push(format!("{} edge{} removed", removed_edges_list.len(), if removed_edges_list.len() == 1 { "" } else { "s" })); }
    GraphDiff {
        new_nodes: added_nodes, removed_nodes, new_edges: added_edges_list, removed_edges: removed_edges_list,
        summary: if parts.is_empty() { "no changes".to_string() } else { parts.join(", ") },
    }
}
