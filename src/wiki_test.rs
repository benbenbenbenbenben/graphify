use crate::wiki::*;
use crate::build::*;
use crate::analyze::*;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn make_graph() -> MyGraph {
    let mut g = MyGraph::default();
    let mut e1 = serde_json::Map::new(); e1.insert("community".to_string(), serde_json::json!(0));
    let mut e2 = serde_json::Map::new(); e2.insert("community".to_string(), serde_json::json!(1));
    let n1 = g.add_node(GraphNode { id: "n1".into(), label: "parse".into(), file_type: "code".into(), source_file: "parser.py".into(), extra: e1.clone() });
    let n2 = g.add_node(GraphNode { id: "n2".into(), label: "validate".into(), file_type: "code".into(), source_file: "parser.py".into(), extra: e1.clone() });
    let n3 = g.add_node(GraphNode { id: "n3".into(), label: "render".into(), file_type: "code".into(), source_file: "renderer.py".into(), extra: e2.clone() });
    let n4 = g.add_node(GraphNode { id: "n4".into(), label: "stream".into(), file_type: "code".into(), source_file: "renderer.py".into(), extra: e2.clone() });
    g.add_edge(n1, n2, GraphEdge { relation: "calls".into(), confidence: "EXTRACTED".into(), source_file: "".into(), extra: Default::default(), original_source: "n1".into(), original_target: "n2".into() });
    g.add_edge(n1, n3, GraphEdge { relation: "references".into(), confidence: "INFERRED".into(), source_file: "".into(), extra: Default::default(), original_source: "n1".into(), original_target: "n3".into() });
    g.add_edge(n3, n4, GraphEdge { relation: "calls".into(), confidence: "EXTRACTED".into(), source_file: "".into(), extra: Default::default(), original_source: "n3".into(), original_target: "n4".into() });
    g
}
fn make_communities() -> HashMap<usize, Vec<String>> { let mut c = HashMap::new(); c.insert(0, vec!["n1".to_string(), "n2".to_string()]); c.insert(1, vec!["n3".to_string(), "n4".to_string()]); c }
fn make_labels() -> HashMap<usize, String> { let mut l = HashMap::new(); l.insert(0, "Parsing Layer".to_string()); l.insert(1, "Rendering Layer".to_string()); l }
fn make_cohesion() -> HashMap<usize, f64> { let mut c = HashMap::new(); c.insert(0, 0.85); c.insert(1, 0.72); c }
fn make_god_nodes() -> Vec<GodNode> { vec![GodNode { id: "n1".to_string(), label: "parse".to_string(), edges: 2 }] }

#[test] fn test_to_wiki_writes_index() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), Some(&make_cohesion()), Some(&make_god_nodes())); assert!(dir.path().join("index.md").exists()); }
#[test] fn test_to_wiki_returns_article_count() { let dir = TempDir::new().unwrap(); let g = make_graph(); let count = to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), Some(&make_cohesion()), Some(&make_god_nodes())); assert_eq!(count, 3); }
#[test] fn test_to_wiki_community_articles_created() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, None); assert!(dir.path().join("Parsing_Layer.md").exists()); assert!(dir.path().join("Rendering_Layer.md").exists()); }
#[test] fn test_to_wiki_god_node_article_created() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, Some(&make_god_nodes())); assert!(dir.path().join("parse.md").exists()); }
#[test] fn test_index_links_all_communities() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, None); let index = fs::read_to_string(dir.path().join("index.md")).unwrap(); assert!(index.contains("[[Parsing Layer]]")); assert!(index.contains("[[Rendering Layer]]")); }
#[test] fn test_index_lists_god_nodes() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, Some(&make_god_nodes())); let index = fs::read_to_string(dir.path().join("index.md")).unwrap(); assert!(index.contains("[[parse]]")); assert!(index.contains("2 connections")); }
#[test] fn test_community_article_has_cross_links() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, None); let parsing = fs::read_to_string(dir.path().join("Parsing_Layer.md")).unwrap(); assert!(parsing.contains("[[Rendering Layer]]")); }
#[test] fn test_community_article_shows_cohesion() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), Some(&make_cohesion()), None); let parsing = fs::read_to_string(dir.path().join("Parsing_Layer.md")).unwrap(); assert!(parsing.contains("cohesion 0.85")); }
#[test] fn test_community_article_has_audit_trail() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, None); let parsing = fs::read_to_string(dir.path().join("Parsing_Layer.md")).unwrap(); assert!(parsing.contains("EXTRACTED")); assert!(parsing.contains("INFERRED")); }
#[test] fn test_god_node_article_links_community() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, Some(&make_god_nodes())); let article = fs::read_to_string(dir.path().join("parse.md")).unwrap(); assert!(article.contains("[[Parsing Layer]]")); }
#[test] fn test_to_wiki_no_labels_uses_fallback() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), None, None, None); assert!(dir.path().join("Community_0.md").exists()); assert!(dir.path().join("Community_1.md").exists()); }
#[test] fn test_article_navigation_footer() { let dir = TempDir::new().unwrap(); let g = make_graph(); to_wiki(&g, &make_communities(), dir.path(), Some(&make_labels()), None, None); let article = fs::read_to_string(dir.path().join("Parsing_Layer.md")).unwrap(); assert!(article.contains("[[index]]")); }
