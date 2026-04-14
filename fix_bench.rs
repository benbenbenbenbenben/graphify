use std::fs;
fn main() {
    let content = fs::read_to_string("tests/fixtures/extraction.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    println!("nodes: {}", v.get("nodes").unwrap().as_array().unwrap().len());
    println!("edges: {}", v.get("edges").unwrap().as_array().unwrap().len());
}
