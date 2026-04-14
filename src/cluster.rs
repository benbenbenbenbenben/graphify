use crate::build::{MyGraph, build_from_json};
use crate::validate::Extraction;
use petgraph::algo::tarjan_scc;
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

pub fn cluster(g: &MyGraph) -> HashMap<usize, Vec<String>> {
    let mut node_to_idx = HashMap::new();
    let mut idx_to_node = HashMap::new();
    let mut nodes = Vec::new();

    for (i, node) in g.node_references().enumerate() {
        let id = node.1.id.clone();
        node_to_idx.insert(id.clone(), i);
        idx_to_node.insert(i, id.clone());
        nodes.push(id);
    }

    if nodes.is_empty() { return HashMap::new(); }
    if g.edge_count() == 0 {
        nodes.sort();
        return nodes.into_iter().enumerate().map(|(i, n)| (i, vec![n])).collect();
    }

    let mut visited = HashSet::new();
    let mut components = Vec::new();

    for node_idx in g.node_indices() {
        if !visited.contains(&node_idx) {
            let mut component = Vec::new();
            let mut stack = vec![node_idx];
            visited.insert(node_idx);

            while let Some(curr) = stack.pop() {
                component.push(g.node_weight(curr).unwrap().id.clone());
                for edge in g.edges(curr) {
                    let next = if edge.source() == curr { edge.target() } else { edge.source() };
                    if visited.insert(next) { stack.push(next); }
                }
            }
            components.push(component);
        }
    }

    let max_size = 10.max((g.node_count() as f64 * 0.25) as usize);
    let mut final_communities = Vec::new();

    for mut comp in components {
        if comp.len() > max_size {
            for chunk in comp.chunks(max_size) {
                let mut c = chunk.to_vec();
                c.sort();
                final_communities.push(c);
            }
        } else {
            comp.sort();
            final_communities.push(comp);
        }
    }

    final_communities.sort_by(|a, b| b.len().cmp(&a.len()));
    final_communities.into_iter().enumerate().collect()
}

pub fn cohesion_score(g: &MyGraph, community_nodes: &[String]) -> f64 {
    let n = community_nodes.len();
    if n <= 1 { return 1.0; }

    let nodes_set: HashSet<&String> = community_nodes.iter().collect();
    let mut actual_edges = 0;

    for edge in g.edge_references() {
        let src_id = &g.node_weight(edge.source()).unwrap().id;
        let tgt_id = &g.node_weight(edge.target()).unwrap().id;

        if nodes_set.contains(src_id) && nodes_set.contains(tgt_id) {
            actual_edges += 1;
        }
    }

    let possible = (n * (n - 1)) / 2;
    if possible > 0 {
        let score = actual_edges as f64 / possible as f64;
        (score * 100.0).round() / 100.0
    } else { 0.0 }
}

pub fn score_all(g: &MyGraph, communities: &HashMap<usize, Vec<String>>) -> HashMap<usize, f64> {
    communities.iter().map(|(&cid, nodes)| (cid, cohesion_score(g, nodes))).collect()
}
