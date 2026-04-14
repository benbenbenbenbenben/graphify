use crate::build::MyGraph;
use crate::analyze::GodNode;
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

fn safe_filename(name: &str) -> String { name.replace('/', "-").replace(' ', "_").replace(':', "-") }

fn cross_community_links(g: &MyGraph, nodes: &[String], own_cid: usize, labels: &HashMap<usize, String>) -> Vec<(String, usize)> {
    let mut counts = HashMap::new();
    let mut id_to_idx = HashMap::new();
    for (idx, node) in g.node_references() { id_to_idx.insert(node.id.clone(), idx); }
    for nid in nodes {
        if let Some(&idx) = id_to_idx.get(nid) {
            for neighbor in g.neighbors(idx) {
                if let Some(ncid) = g.node_weight(neighbor).unwrap().extra.get("community").and_then(|v| v.as_u64()).map(|v| v as usize) {
                    if ncid != own_cid {
                        let label = labels.get(&ncid).cloned().unwrap_or_else(|| format!("Community {}", ncid));
                        *counts.entry(label).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

fn community_article(
    g: &MyGraph, cid: usize, nodes: &[String], label: &str, labels: &HashMap<usize, String>, cohesion: Option<f64>,
) -> String {
    let mut id_to_idx = HashMap::new();
    for (idx, n) in g.node_references() { id_to_idx.insert(n.id.clone(), idx); }
    let mut top_nodes = nodes.to_vec();
    top_nodes.sort_by_cached_key(|nid| if let Some(&idx) = id_to_idx.get(nid) { std::cmp::Reverse(g.edges(idx).count()) } else { std::cmp::Reverse(0) });
    let cross = cross_community_links(g, nodes, cid, labels);
    let mut conf_counts = HashMap::new();
    for nid in nodes {
        if let Some(&idx) = id_to_idx.get(nid) {
            for edge in g.edges(idx) { *conf_counts.entry(edge.weight().confidence.clone()).or_insert(0) += 1; }
        }
    }
    let total_edges: usize = conf_counts.values().sum();
    let total_edges = if total_edges == 0 { 1 } else { total_edges };
    let mut sources_set = HashSet::new();
    for nid in nodes {
        if let Some(&idx) = id_to_idx.get(nid) {
            let src = &g.node_weight(idx).unwrap().source_file;
            if !src.is_empty() { sources_set.insert(src.clone()); }
        }
    }
    let mut sources: Vec<_> = sources_set.into_iter().collect();
    sources.sort();
    let mut lines = vec![format!("# {}", label), "".to_string()];
    let mut meta_parts = vec![format!("{} nodes", nodes.len())];
    if let Some(coh) = cohesion { meta_parts.push(format!("cohesion {:.2}", coh)); }
    lines.push(format!("> {}", meta_parts.join(" · ")));
    lines.push("".to_string());
    lines.push("## Key Concepts".to_string());
    lines.push("".to_string());
    for nid in top_nodes.iter().take(25) {
        if let Some(&idx) = id_to_idx.get(nid) {
            let d = g.node_weight(idx).unwrap();
            let node_label = if d.label.is_empty() { nid.clone() } else { d.label.clone() };
            let src = &d.source_file;
            let degree = g.edges(idx).count();
            let src_str = if src.is_empty() { "".to_string() } else { format!(" — `{}`", src) };
            lines.push(format!("- **{}** ({} connections){}", node_label, degree, src_str));
        }
    }
    let remaining = nodes.len().saturating_sub(25);
    if remaining > 0 { lines.push(format!("- *... and {} more nodes in this community*", remaining)); }
    lines.push("".to_string());
    lines.push("## Relationships".to_string());
    lines.push("".to_string());
    if !cross.is_empty() {
        for (other_label, count) in cross.iter().take(12) { lines.push(format!("- [[{}]] ({} shared connections)", other_label, count)); }
    } else { lines.push("- No strong cross-community connections detected".to_string()); }
    lines.push("".to_string());
    if !sources.is_empty() {
        lines.push("## Source Files".to_string()); lines.push("".to_string());
        for src in sources.iter().take(20) { lines.push(format!("- `{}`", src)); }
        lines.push("".to_string());
    }
    lines.push("## Audit Trail".to_string()); lines.push("".to_string());
    for conf in &["EXTRACTED", "INFERRED", "AMBIGUOUS"] {
        let n = conf_counts.get(*conf).copied().unwrap_or(0);
        let pct = ((n as f64 / total_edges as f64) * 100.0).round() as usize;
        lines.push(format!("- {}: {} ({}%)", conf, n, pct));
    }
    lines.push("".to_string());
    lines.push("---".to_string()); lines.push("".to_string()); lines.push("*Part of the graphify knowledge wiki. See [[index]] to navigate.*".to_string());
    lines.join("\n")
}

fn god_node_article(g: &MyGraph, nid: &str, labels: &HashMap<usize, String>) -> String {
    let mut lines = Vec::new();
    let mut id_to_idx = HashMap::new();
    for (idx, n) in g.node_references() { id_to_idx.insert(n.id.clone(), idx); }
    if let Some(&idx) = id_to_idx.get(nid) {
        let d = g.node_weight(idx).unwrap();
        let node_label = if d.label.is_empty() { nid.to_string() } else { d.label.clone() };
        let src = &d.source_file;
        let cid = d.extra.get("community").and_then(|v| v.as_u64()).map(|v| v as usize);
        let community_name = cid.map(|c| labels.get(&c).cloned().unwrap_or_else(|| format!("Community {}", c)));
        lines.push(format!("# {}", node_label)); lines.push("".to_string());
        lines.push(format!("> God node · {} connections · `{}`", g.edges(idx).count(), src)); lines.push("".to_string());
        if let Some(cname) = community_name { lines.push(format!("**Community:** [[{}]]", cname)); lines.push("".to_string()); }
        let mut by_relation: HashMap<String, Vec<String>> = HashMap::new();
        let mut neighbors: Vec<_> = g.neighbors(idx).collect();
        neighbors.sort_by_cached_key(|&n| std::cmp::Reverse(g.edges(n).count()));
        for neighbor in neighbors {
            let nd = g.node_weight(neighbor).unwrap();
            if let Some(edge) = g.edges(idx).find(|e| e.target() == neighbor || e.source() == neighbor) {
                let rel = if edge.weight().relation.is_empty() { "related".to_string() } else { edge.weight().relation.clone() };
                let neighbor_label = if nd.label.is_empty() { nd.id.clone() } else { nd.label.clone() };
                let conf = &edge.weight().confidence;
                let conf_str = if conf.is_empty() { "".to_string() } else { format!(" `{}`", conf) };
                by_relation.entry(rel).or_default().push(format!("[[{}]] {}", neighbor_label, conf_str));
            }
        }
        lines.push("## Connections by Relation".to_string()); lines.push("".to_string());
        let mut sorted_rels: Vec<_> = by_relation.keys().collect(); sorted_rels.sort();
        for rel in sorted_rels {
            lines.push(format!("### {}", rel));
            for t in by_relation.get(rel).unwrap().iter().take(20) { lines.push(format!("- {}", t)); }
            lines.push("".to_string());
        }
    }
    lines.push("---".to_string()); lines.push("".to_string()); lines.push("*Part of the graphify knowledge wiki. See [[index]] to navigate.*".to_string());
    lines.join("\n")
}

fn index_md(communities: &HashMap<usize, Vec<String>>, labels: &HashMap<usize, String>, god_nodes_data: &[GodNode], total_nodes: usize, total_edges: usize) -> String {
    let mut lines = vec![
        "# Knowledge Graph Index".to_string(), "".to_string(),
        "> Auto-generated by graphify. Start here — read community articles for context, then drill into god nodes for detail.".to_string(), "".to_string(),
        format!("**{} nodes · {} edges · {} communities**", total_nodes, total_edges, communities.len()), "".to_string(),
        "---".to_string(), "".to_string(), "## Communities".to_string(), "(sorted by size, largest first)".to_string(), "".to_string(),
    ];
    let mut sorted_comms: Vec<_> = communities.iter().collect();
    sorted_comms.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (cid, nodes) in sorted_comms {
        let label = labels.get(cid).cloned().unwrap_or_else(|| format!("Community {}", cid));
        lines.push(format!("- [[{}]] — {} nodes", label, nodes.len()));
    }
    lines.push("".to_string());
    if !god_nodes_data.is_empty() {
        lines.push("## God Nodes".to_string()); lines.push("(most connected concepts — the load-bearing abstractions)".to_string()); lines.push("".to_string());
        for node in god_nodes_data { lines.push(format!("- [[{}]] — {} connections", node.label, node.edges)); }
        lines.push("".to_string());
    }
    lines.push("---".to_string()); lines.push("".to_string()); lines.push("*Generated by [graphify](https://github.com/safishamsi/graphify)*".to_string());
    lines.join("\n")
}

pub fn to_wiki(
    g: &MyGraph, communities: &HashMap<usize, Vec<String>>, output_dir: &Path,
    community_labels: Option<&HashMap<usize, String>>, cohesion: Option<&HashMap<usize, f64>>, god_nodes_data: Option<&[GodNode]>,
) -> usize {
    let _ = fs::create_dir_all(output_dir);
    let empty_labels = HashMap::new(); let labels = community_labels.unwrap_or(&empty_labels);
    let mut fallback_labels = HashMap::new();
    let effective_labels = if community_labels.is_none() {
        for cid in communities.keys() { fallback_labels.insert(*cid, format!("Community {}", cid)); }
        &fallback_labels
    } else { labels };
    let empty_cohesion = HashMap::new(); let coh = cohesion.unwrap_or(&empty_cohesion);
    let empty_gods = Vec::new(); let gods = god_nodes_data.unwrap_or(&empty_gods);
    let mut count = 0;
    for (cid, nodes) in communities {
        let label = effective_labels.get(cid).cloned().unwrap_or_else(|| format!("Community {}", cid));
        let article = community_article(g, *cid, nodes, &label, effective_labels, coh.get(cid).copied());
        let _ = fs::write(output_dir.join(format!("{}.md", safe_filename(&label))), article);
        count += 1;
    }
    let mut id_to_idx = HashMap::new();
    for (idx, n) in g.node_references() { id_to_idx.insert(n.id.clone(), idx); }
    for node_data in gods {
        let nid = &node_data.id;
        if id_to_idx.contains_key(nid) {
            let article = god_node_article(g, nid, effective_labels);
            let _ = fs::write(output_dir.join(format!("{}.md", safe_filename(&node_data.label))), article);
            count += 1;
        }
    }
    let index = index_md(communities, effective_labels, gods, g.node_count(), g.edge_count());
    let _ = fs::write(output_dir.join("index.md"), index);
    count
}
