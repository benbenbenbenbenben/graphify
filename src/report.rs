use crate::build::MyGraph;
use crate::analyze::{GodNode, Surprise};
use crate::detect::DetectResult;
use chrono::Local;
use std::collections::HashMap;

fn is_concept_node(node: &crate::build::GraphNode) -> bool {
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
    use petgraph::visit::EdgeRef;
    if node.label.ends_with("()") && g.edges(node_idx).count() <= 1 { return true; }
    false
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub r#type: String,
    pub question: Option<String>,
    pub why: String,
}

pub fn generate(
    g: &MyGraph,
    communities: &HashMap<usize, Vec<String>>,
    cohesion_scores: &HashMap<usize, f64>,
    community_labels: &HashMap<usize, String>,
    god_node_list: &[GodNode],
    surprise_list: &[Surprise],
    detection_result: &DetectResult,
    token_cost: &(usize, usize),
    root: &str,
    suggested_questions: Option<&[Question]>,
) -> String {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let mut ext_count = 0; let mut inf_count = 0; let mut amb_count = 0;
    use petgraph::visit::EdgeRef;
    let total_edges = g.edge_count().max(1);

    let mut inf_scores = Vec::new();
    let mut inf_edges_count = 0;
    let mut ambiguous_edges = Vec::new();

    for edge in g.edge_references() {
        let conf = edge.weight().confidence.as_str();
        match conf {
            "EXTRACTED" => ext_count += 1,
            "INFERRED" => {
                inf_count += 1; inf_edges_count += 1;
                if let Some(score) = edge.weight().extra.get("confidence_score").and_then(|v| v.as_f64()) {
                    inf_scores.push(score);
                } else { inf_scores.push(0.5); }
            },
            "AMBIGUOUS" => {
                amb_count += 1;
                let u_label = &g.node_weight(edge.source()).unwrap().label;
                let v_label = &g.node_weight(edge.target()).unwrap().label;
                ambiguous_edges.push((u_label.clone(), v_label.clone(), edge.weight().clone()));
            },
            _ => ext_count += 1,
        }
    }

    let ext_pct = (ext_count as f64 / total_edges as f64 * 100.0).round() as usize;
    let inf_pct = (inf_count as f64 / total_edges as f64 * 100.0).round() as usize;
    let amb_pct = (amb_count as f64 / total_edges as f64 * 100.0).round() as usize;

    let inf_avg = if !inf_scores.is_empty() {
        let sum: f64 = inf_scores.iter().sum();
        Some(sum / inf_scores.len() as f64)
    } else { None };

    let mut lines = vec![
        format!("# Graph Report - {}  ({})", root, today),
        "".to_string(), "## Corpus Check".to_string(),
    ];

    if let Some(warn) = &detection_result.warning {
        lines.push(format!("- {}", warn));
    } else {
        lines.push(format!("- {} files · ~{} words", detection_result.total_files, detection_result.total_words));
        lines.push("- Verdict: corpus is large enough that graph structure adds value.".to_string());
    }

    let inf_part = if let Some(avg) = inf_avg {
        format!(" · INFERRED: {} edges (avg confidence: {:.2})", inf_edges_count, avg)
    } else { "".to_string() };

    lines.extend_from_slice(&[
        "".to_string(), "## Summary".to_string(),
        format!("- {} nodes · {} edges · {} communities detected", g.node_count(), g.edge_count(), communities.len()),
        format!("- Extraction: {}% EXTRACTED · {}% INFERRED · {}% AMBIGUOUS{}", ext_pct, inf_pct, amb_pct, inf_part),
        format!("- Token cost: {} input · {} output", token_cost.0, token_cost.1),
        "".to_string(), "## God Nodes (most connected - your core abstractions)".to_string(),
    ]);

    for (i, node) in god_node_list.iter().enumerate() {
        lines.push(format!("{}. `{}` - {} edges", i + 1, node.label, node.edges));
    }

    lines.push("".to_string());
    lines.push("## Surprising Connections (you probably didn't know these)".to_string());

    if !surprise_list.is_empty() {
        for s in surprise_list {
            let relation = &s.relation;
            let note = s.note.as_deref().unwrap_or("");
            let files = if s.source_files.len() >= 2 { (&s.source_files[0], &s.source_files[1]) } else { (&String::new(), &String::new()) };
            let conf_tag = s.confidence.clone();
            let sem_tag = if relation == "semantically_similar_to" { " [semantically similar]" } else { "" };
            lines.push(format!("- `{}` --{}--> `{}`  [{}]{}", s.source, relation, s.target, conf_tag, sem_tag));
            let note_str = if !note.is_empty() { format!("  _{}_", note) } else { "".to_string() };
            lines.push(format!("  {} → {}{}", files.0, files.1, note_str));
        }
    } else {
        lines.push("- None detected - all connections are within the same source files.".to_string());
    }

    lines.push("".to_string());
    lines.push("## Communities".to_string());

    let mut sorted_comms: Vec<_> = communities.iter().collect();
    sorted_comms.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (cid, nodes) in sorted_comms {
        let label = community_labels.get(cid).cloned().unwrap_or_else(|| format!("Community {}", cid));
        let score = cohesion_scores.get(cid).copied().unwrap_or(0.0);
        let mut real_nodes = Vec::new();
        for node_id in nodes {
            if let Some(idx) = g.node_indices().find(|i| g.node_weight(*i).unwrap().id == *node_id) {
                if !is_file_node(g, idx) {
                    real_nodes.push(g.node_weight(idx).unwrap().label.clone());
                }
            }
        }
        let display: Vec<String> = real_nodes.iter().take(8).cloned().collect();
        let suffix = if real_nodes.len() > 8 { format!(" (+{} more)", real_nodes.len() - 8) } else { "".to_string() };
        lines.push("".to_string());
        lines.push(format!("### Community {} - \"{}\"", cid, label));
        lines.push(format!("Cohesion: {}", score));
        lines.push(format!("Nodes ({}): {}{}", real_nodes.len(), display.join(", "), suffix));
    }

    if !ambiguous_edges.is_empty() {
        lines.push("".to_string());
        lines.push("## Ambiguous Edges - Review These".to_string());
        for (u, v, d) in ambiguous_edges {
            lines.push(format!("- `{}` → `{}`  [AMBIGUOUS]", u, v));
            lines.push(format!("  {} · relation: {}", d.source_file, d.relation));
        }
    }

    let mut isolated = Vec::new();
    for idx in g.node_indices() {
        let node = g.node_weight(idx).unwrap();
        if g.edges(idx).count() <= 1 && !is_file_node(g, idx) && !is_concept_node(node) {
            isolated.push(node.label.clone());
        }
    }

    let thin_communities: Vec<_> = communities.iter().filter(|(_, n)| n.len() < 3).collect();
    let gap_count = isolated.len() + thin_communities.len();

    if gap_count > 0 || amb_pct > 20 {
        lines.push("".to_string());
        lines.push("## Knowledge Gaps".to_string());
        if !isolated.is_empty() {
            let display: Vec<String> = isolated.iter().take(5).map(|s| format!("`{}`", s)).collect();
            let suffix = if isolated.len() > 5 { format!(" (+{} more)", isolated.len() - 5) } else { "".to_string() };
            lines.push(format!("- **{} isolated node(s):** {}{}", isolated.len(), display.join(", "), suffix));
            lines.push("  These have ≤1 connection - possible missing edges or undocumented components.".to_string());
        }
        if !thin_communities.is_empty() {
            for (cid, nodes) in thin_communities {
                let label = community_labels.get(cid).cloned().unwrap_or_else(|| format!("Community {}", cid));
                let mut n_labels = Vec::new();
                for node_id in nodes {
                    if let Some(idx) = g.node_indices().find(|i| g.node_weight(*i).unwrap().id == *node_id) {
                        n_labels.push(format!("`{}`", g.node_weight(idx).unwrap().label));
                    }
                }
                lines.push(format!("- **Thin community `{}`** ({} nodes): {}", label, nodes.len(), n_labels.join(", ")));
                lines.push("  Too small to be a meaningful cluster - may be noise or needs more connections extracted.".to_string());
            }
        }
        if amb_pct > 20 {
            lines.push(format!("- **High ambiguity: {}% of edges are AMBIGUOUS.** Review the Ambiguous Edges section above.", amb_pct));
        }
    }

    if let Some(questions) = suggested_questions {
        lines.push("".to_string());
        lines.push("## Suggested Questions".to_string());
        let no_signal = questions.len() == 1 && questions[0].r#type == "no_signal";
        if no_signal {
            lines.push(format!("_{}_", questions[0].why));
        } else {
            lines.push("_Questions this graph is uniquely positioned to answer:_".to_string());
            lines.push("".to_string());
            for q in questions {
                if let Some(qs) = &q.question {
                    lines.push(format!("- **{}**", qs));
                    lines.push(format!("  _{}_", q.why));
                }
            }
        }
    }
    lines.join("\n")
}
