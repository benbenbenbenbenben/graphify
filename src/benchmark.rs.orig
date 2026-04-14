use crate::build::MyGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

const CHARS_PER_TOKEN: usize = 4;

fn estimate_tokens(text: &str) -> usize {
    1.max(text.len() / CHARS_PER_TOKEN)
}

pub fn query_subgraph_tokens(g: &MyGraph, question: &str, depth: usize) -> usize {
    let terms: Vec<String> = question.split_whitespace()
        .filter(|t| t.len() > 2)
        .map(|t| t.to_lowercase())
        .collect();

    let mut scored = Vec::new();
    for node_idx in g.node_indices() {
        let label = g.node_weight(node_idx).unwrap().label.to_lowercase();
        let score = terms.iter().filter(|t| label.contains(t.as_str())).count();
        if score > 0 {
            scored.push((score, node_idx));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let start_nodes: Vec<_> = scored.into_iter().take(3).map(|(_, idx)| idx).collect();

    if start_nodes.is_empty() {
        return 0;
    }

    let mut visited: HashSet<_> = start_nodes.iter().cloned().collect();
    let mut frontier: HashSet<_> = start_nodes.into_iter().collect();
    let mut edges_seen = Vec::new();

    for _ in 0..depth {
        let mut next_frontier = HashSet::new();
        for &n in &frontier {
            for neighbor in g.neighbors(n) {
                if !visited.contains(&neighbor) {
                    next_frontier.insert(neighbor);
                    let (u, v) = if n < neighbor { (n, neighbor) } else { (neighbor, n) };
                    edges_seen.push((u, v));
                }
            }
        }
        visited.extend(&next_frontier);
        frontier = next_frontier;
    }

    let mut lines = Vec::new();
    for &nid in &visited {
        let d = g.node_weight(nid).unwrap();
        let label = if d.label.is_empty() { d.id.clone() } else { d.label.clone() };
        let loc = d.extra.get("source_location").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("NODE {} src={} loc={}", label, d.source_file, loc));
    }

    for (u, v) in edges_seen {
        if visited.contains(&u) && visited.contains(&v) {
            if let Some(edge) = g.edges(u).find(|e| {
                use petgraph::visit::EdgeRef;
                e.target() == v || e.source() == v
            }) {
                use petgraph::visit::EdgeRef;
                let u_label = &g.node_weight(u).unwrap().label;
                let v_label = &g.node_weight(v).unwrap().label;
                let u_label = if u_label.is_empty() { &g.node_weight(u).unwrap().id } else { u_label };
                let v_label = if v_label.is_empty() { &g.node_weight(v).unwrap().id } else { v_label };
                lines.push(format!("EDGE {} --{}--> {}", u_label, edge.weight().relation, v_label));
            }
        }
    }

    estimate_tokens(&lines.join("\n"))
}

pub const SAMPLE_QUESTIONS: &[&str] = &[
    "how does authentication work",
    "what is the main entry point",
    "how are errors handled",
    "what connects the data layer to the api",
    "what are the core abstractions",
];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionResult {
    pub question: String,
    pub query_tokens: usize,
    pub reduction: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkResult {
    pub error: Option<String>,
    pub corpus_tokens: usize,
    pub corpus_words: usize,
    pub nodes: usize,
    pub edges: usize,
    pub avg_query_tokens: usize,
    pub reduction_ratio: f64,
    pub per_question: Vec<QuestionResult>,
}

pub fn run_benchmark(
    graph_path: &str,
    corpus_words_opt: Option<usize>,
    questions_opt: Option<&[&str]>,
) -> BenchmarkResult {
    let content = match fs::read_to_string(graph_path) {
        Ok(c) => c,
        Err(_) => return BenchmarkResult { error: Some("Failed to read graph file".into()), corpus_tokens: 0, corpus_words: 0, nodes: 0, edges: 0, avg_query_tokens: 0, reduction_ratio: 0.0, per_question: vec![] }
    };

    let mut ext = crate::validate::Extraction::default();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
            for n in nodes {
                if let Ok(node) = serde_json::from_value(n.clone()) { ext.nodes.push(node); }
                else if let Ok(mut node_map) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(n.clone()) {
                    let id = node_map.remove("id").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let label = node_map.remove("label").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let file_type = node_map.remove("file_type").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "code".into());
                    let source_file = node_map.remove("source_file").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    ext.nodes.push(crate::validate::Node { id, label, file_type, source_file, extra: node_map });
                }
            }
        }
        let edges_arr = v.get("links").or_else(|| v.get("edges")).and_then(|n| n.as_array());
        if let Some(arr) = edges_arr {
            for l in arr {
                if let Ok(edge) = serde_json::from_value(l.clone()) { ext.edges.push(edge); }
                else if let Ok(mut edge_map) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(l.clone()) {
                    let source = edge_map.remove("source").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let target = edge_map.remove("target").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let relation = edge_map.remove("relation").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let confidence = edge_map.remove("confidence").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "EXTRACTED".into());
                    let source_file = edge_map.remove("source_file").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    ext.edges.push(crate::validate::Edge { source, target, relation, confidence, source_file, extra: edge_map });
                }
            }
        }
    } else {
        return BenchmarkResult { error: Some("Failed to parse graph JSON".into()), corpus_tokens: 0, corpus_words: 0, nodes: 0, edges: 0, avg_query_tokens: 0, reduction_ratio: 0.0, per_question: vec![] };
    }

    let g = crate::build::build_from_json(&ext);

    let corpus_words = corpus_words_opt.unwrap_or_else(|| g.node_count() * 50);
    let corpus_tokens = (corpus_words * 100) / 75;

    let qs = questions_opt.unwrap_or(SAMPLE_QUESTIONS);
    let mut per_question = Vec::new();

    for &q in qs {
        let qt = query_subgraph_tokens(&g, q, 3);
        if qt > 0 {
            let reduction = (corpus_tokens as f64 / qt as f64 * 10.0).round() / 10.0;
            per_question.push(QuestionResult {
                question: q.to_string(),
                query_tokens: qt,
                reduction,
            });
        } else {
            // Python benchmark does not require this specifically but to fulfill our test "test_run_benchmark_per_question_list"
            // if we use matching words we shouldn't get 0, but if we do, skip it rather than returning 0 unless needed.
        }
    }

    if per_question.is_empty() {
        return BenchmarkResult {
            error: Some("No matching nodes found for sample questions. Build the graph first.".into()),
            corpus_tokens: 0, corpus_words: 0, nodes: 0, edges: 0, avg_query_tokens: 0, reduction_ratio: 0.0, per_question: vec![]
        };
    }

    let avg_query_tokens = per_question.iter().map(|p| p.query_tokens).sum::<usize>() / per_question.len();
    let reduction_ratio = if avg_query_tokens > 0 {
        (corpus_tokens as f64 / avg_query_tokens as f64 * 10.0).round() / 10.0
    } else {
        0.0
    };

    BenchmarkResult {
        error: None,
        corpus_tokens,
        corpus_words,
        nodes: g.node_count(),
        edges: g.edge_count(),
        avg_query_tokens,
        reduction_ratio,
        per_question,
    }
}

pub fn print_benchmark(result: &BenchmarkResult) {
    if let Some(err) = &result.error {
        println!("Benchmark error: {}", err);
        return;
    }

    println!("\ngraphify token reduction benchmark");
    println!("{}", "─".repeat(50));
    println!("  Corpus:          {} words → ~{} tokens (naive)", result.corpus_words, result.corpus_tokens);
    println!("  Graph:           {} nodes, {} edges", result.nodes, result.edges);
    println!("  Avg query cost:  ~{} tokens", result.avg_query_tokens);
    println!("  Reduction:       {}x fewer tokens per query", result.reduction_ratio);
    println!("\n  Per question:");
    for p in &result.per_question {
        let q_short = if p.question.len() > 55 { &p.question[..55] } else { &p.question };
        println!("    [{}x] {}", p.reduction, q_short);
    }
    println!();
}
