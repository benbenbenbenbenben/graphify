use graphify::detect::{detect, detect_incremental};
use graphify::cache::{check_semantic_cache, save_semantic_cache};
use graphify::extract::{collect_files, extract};
use graphify::build::build;
use graphify::cluster::{cluster, score_all};
use graphify::analyze::{god_nodes, surprising_connections};
use graphify::export::{to_json, to_html, to_graphml, to_cypher};
use graphify::report::generate;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("graphify 0.3.12 (Rust port)");
        return;
    }

    if args.len() < 2 {
        println!("Usage: graphify <path> [options]");
        return;
    }

    let command = &args[1];
    if command == "hook" {
        println!("Git hook logic triggered");
        return;
    } else if command == "query" {
        println!("Query logic triggered");
        return;
    } else if command == "add" {
        println!("Ingest logic triggered");
        return;
    } else if command == "install" {
        println!("Platform install logic triggered");
        return;
    } else if command == "uninstall" {
        println!("Platform uninstall logic triggered");
        return;
    }

    let root_path = Path::new(&args[1]);
    let out_dir = root_path.join("graphify-out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let detect_result = if args.contains(&"--update".to_string()) {
        let mut d = detect_incremental(root_path, out_dir.join("manifest.json").to_str().unwrap());
        d.detect
    } else {
        detect(root_path, false)
    };

    println!("[graphify] Found {} files ({} code)", detect_result.total_files, detect_result.files.get("code").map_or(0, |v| v.len()));

    let mut all_extractions = Vec::new();
    let mut input_tokens = 0;
    let mut output_tokens = 0;

    let code_files = collect_files(root_path, false);
    if !code_files.is_empty() {
        let code_ext = extract(&code_files);
        input_tokens += code_ext.input_tokens;
        output_tokens += code_ext.output_tokens;
        all_extractions.push(code_ext);
    }

    let semantic_files: Vec<String> = detect_result.files.get("document").unwrap_or(&vec![]).iter()
        .chain(detect_result.files.get("paper").unwrap_or(&vec![]).iter())
        .chain(detect_result.files.get("image").unwrap_or(&vec![]).iter())
        .cloned()
        .collect();

    if !semantic_files.is_empty() {
        let (c_nodes, c_edges, c_hyperedges, _) = check_semantic_cache(&semantic_files, root_path);
        let mut semantic_ext = graphify::validate::Extraction::default();
        semantic_ext.nodes.extend(c_nodes);
        semantic_ext.edges.extend(c_edges);
        all_extractions.push(semantic_ext);
    }

    let g = build(&all_extractions);
    if g.node_count() == 0 {
        println!("[graphify] No files analyzed. Check .graphifyignore.");
        return;
    }

    let communities = cluster(&g);
    let cohesion = score_all(&g, &communities);
    let gods = god_nodes(&g, 10);
    let surprises = surprising_connections(&g, Some(&communities), 5);

    let mut labels = std::collections::HashMap::new();
    for cid in communities.keys() {
        labels.insert(*cid, format!("Community {}", cid));
    }

    let tokens = (input_tokens as usize, output_tokens as usize);
    let report = generate(&g, &communities, &cohesion, &labels, &gods, &surprises, &detect_result, &tokens, root_path.to_str().unwrap(), None);

    std::fs::write(out_dir.join("GRAPH_REPORT.md"), report).unwrap();
    to_json(&g, &communities, out_dir.join("graph.json").to_str().unwrap());

    if !args.contains(&"--no-viz".to_string()) {
        let _ = to_html(&g, &communities, out_dir.join("graph.html").to_str().unwrap(), Some(&labels));
    }
    if args.contains(&"--graphml".to_string()) {
        to_graphml(&g, &communities, out_dir.join("graph.graphml").to_str().unwrap());
    }
    if args.contains(&"--neo4j".to_string()) {
        to_cypher(&g, out_dir.join("cypher.txt").to_str().unwrap());
    }

    println!("[graphify] Graph built: {} nodes, {} edges.", g.node_count(), g.edge_count());
}
