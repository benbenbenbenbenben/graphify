use crate::build::MyGraph;
use crate::security::sanitize_label;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::EdgeRef;
use serde_json::json;
use regex::Regex;

const COMMUNITY_COLORS: &[&str] = &[
    "#4E79A7", "#F28E2B", "#E15759", "#76B7B2", "#59A14F",
    "#EDC948", "#B07AA1", "#FF9DA7", "#9C755F", "#BAB0AC",
];

const MAX_NODES_FOR_VIZ: usize = 5000;

fn html_styles() -> &'static str {
    r#"<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0f0f1a; color: #e0e0e0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; display: flex; height: 100vh; overflow: hidden; }
  #graph { flex: 1; }
  #sidebar { width: 280px; background: #1a1a2e; border-left: 1px solid #2a2a4e; display: flex; flex-direction: column; overflow: hidden; }
  #search-wrap { padding: 12px; border-bottom: 1px solid #2a2a4e; }
  #search { width: 100%; background: #0f0f1a; border: 1px solid #3a3a5e; color: #e0e0e0; padding: 7px 10px; border-radius: 6px; font-size: 13px; outline: none; }
  #search:focus { border-color: #4E79A7; }
  #search-results { max-height: 140px; overflow-y: auto; padding: 4px 12px; border-bottom: 1px solid #2a2a4e; display: none; }
  .search-item { padding: 4px 6px; cursor: pointer; border-radius: 4px; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .search-item:hover { background: #2a2a4e; }
  #info-panel { padding: 14px; border-bottom: 1px solid #2a2a4e; min-height: 140px; }
  #info-panel h3 { font-size: 13px; color: #aaa; margin-bottom: 8px; text-transform: uppercase; letter-spacing: 0.05em; }
  #info-content { font-size: 13px; color: #ccc; line-height: 1.6; }
  #info-content .field { margin-bottom: 5px; }
  #info-content .field b { color: #e0e0e0; }
  #info-content .empty { color: #555; font-style: italic; }
  .neighbor-link { display: block; padding: 2px 6px; margin: 2px 0; border-radius: 3px; cursor: pointer; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; border-left: 3px solid #333; }
  .neighbor-link:hover { background: #2a2a4e; }
  #neighbors-list { max-height: 160px; overflow-y: auto; margin-top: 4px; }
  #legend-wrap { flex: 1; overflow-y: auto; padding: 12px; }
  #legend-wrap h3 { font-size: 13px; color: #aaa; margin-bottom: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
  .legend-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; cursor: pointer; border-radius: 4px; font-size: 12px; }
  .legend-item:hover { background: #2a2a4e; padding-left: 4px; }
  .legend-item.dimmed { opacity: 0.35; }
  .legend-dot { width: 12px; height: 12px; border-radius: 50%; flex-shrink: 0; }
  .legend-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legend-count { color: #666; font-size: 11px; }
  #stats { padding: 10px 14px; border-top: 1px solid #2a2a4e; font-size: 11px; color: #555; }
</style>"#
}

fn hyperedge_script(hyperedges_json: &str) -> String {
    format!(r#"<script>
const hyperedges = {};
function drawHyperedges() {{
    const canvas = network.canvas.frame.canvas;
    const ctx = canvas.getContext('2d');
    hyperedges.forEach(h => {{
        const positions = h.nodes.map(nid => network.getPositions([nid])[nid]).filter(p => p !== undefined);
        if (positions.length < 2) return;
        ctx.save();
        ctx.globalAlpha = 0.12; ctx.fillStyle = '#6366f1'; ctx.strokeStyle = '#6366f1'; ctx.lineWidth = 2;
        ctx.beginPath();
        const scale = network.getScale();
        const offset = network.getViewPosition();
        const toCanvas = (p) => ({{ x: (p.x - offset.x) * scale + canvas.width / 2, y: (p.y - offset.y) * scale + canvas.height / 2 }});
        const pts = positions.map(toCanvas);
        const cx = pts.reduce((s, p) => s + p.x, 0) / pts.length;
        const cy = pts.reduce((s, p) => s + p.y, 0) / pts.length;
        const expanded = pts.map(p => ({{ x: cx + (p.x - cx) * 1.15, y: cy + (p.y - cy) * 1.15 }});
        ctx.moveTo(expanded[0].x, expanded[0].y);
        expanded.slice(1).forEach(p => ctx.lineTo(p.x, p.y));
        ctx.closePath(); ctx.fill(); ctx.globalAlpha = 0.4; ctx.stroke();
        ctx.globalAlpha = 0.8; ctx.fillStyle = '#4f46e5'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText(h.label || h.id, cx, cy - 5);
        ctx.restore();
    }});
}}
network.on('afterDrawing', drawHyperedges);
</script>"#, hyperedges_json)
}

fn html_script(nodes_json: &str, edges_json: &str, legend_json: &str) -> String {
    format!(r#"<script>
const RAW_NODES = {};
const RAW_EDGES = {};
const LEGEND = {};

const nodesDS = new vis.DataSet(RAW_NODES.map(n => ({{
  id: n.id, label: n.label, color: n.color, size: n.size,
  font: n.font, title: n.title,
  _community: n.community, _community_name: n.community_name,
  _source_file: n.source_file, _file_type: n.file_type, _degree: n.degree,
}})));

const edgesDS = new vis.DataSet(RAW_EDGES.map((e, i) => ({{
  id: i, from: e.from, to: e.to, label: '', title: e.title, dashes: e.dashes, width: e.width, color: e.color,
  arrows: {{ to: {{ enabled: true, scaleFactor: 0.5 }} }},
}})));

const container = document.getElementById('graph');
const network = new vis.Network(container, {{ nodes: nodesDS, edges: edgesDS }}, {{
  physics: {{
    enabled: true, solver: 'forceAtlas2Based',
    forceAtlas2Based: {{ gravitationalConstant: -60, centralGravity: 0.005, springLength: 120, springConstant: 0.08, damping: 0.4, avoidOverlap: 0.8 }},
    stabilization: {{ iterations: 200, fit: true }},
  }},
  interaction: {{ hover: true, tooltipDelay: 100, hideEdgesOnDrag: true, navigationButtons: false, keyboard: false }},
  nodes: {{ shape: 'dot', borderWidth: 1.5 }},
  edges: {{ smooth: {{ type: 'continuous', roundness: 0.2 }}, selectionWidth: 3 }},
}});

network.once('stabilizationIterationsDone', () => {{ network.setOptions({{ physics: {{ enabled: false }} }}); }});

function showInfo(nodeId) {{
  const n = nodesDS.get(nodeId);
  if (!n) return;
  const neighborIds = network.getConnectedNodes(nodeId);
  const neighborItems = neighborIds.map(nid => {{
    const nb = nodesDS.get(nid);
    const color = nb ? nb.color.background : '#555';
    return `<span class="neighbor-link" style="border-left-color:${{color}}" onclick="focusNode('${{nid}}')">${{nb ? nb.label : nid}}</span>`;
  }}).join('');
  document.getElementById('info-content').innerHTML = `
    <div class="field"><b>${{n.label}}</b></div>
    <div class="field">Type: ${{n._file_type || 'unknown'}}</div>
    <div class="field">Community: ${{n._community_name}}</div>
    <div class="field">Source: ${{n._source_file || '-'}}</div>
    <div class="field">Degree: ${{n._degree}}</div>
    ${{neighborIds.length ? `<div class="field" style="margin-top:8px;color:#aaa;font-size:11px">Neighbors (${{neighborIds.length}})</div><div id="neighbors-list">${{neighborItems}}</div>` : ''}}
  `;
}}

function focusNode(nodeId) {{
  network.focus(nodeId, {{ scale: 1.4, animation: true }});
  network.selectNodes([nodeId]);
  showInfo(nodeId);
}}

network.on('click', params => {{
  if (params.nodes.length > 0) showInfo(params.nodes[0]);
  else document.getElementById('info-content').innerHTML = '<span class="empty">Click a node to inspect it</span>';
}});

const searchInput = document.getElementById('search');
const searchResults = document.getElementById('search-results');
searchInput.addEventListener('input', () => {{
  const q = searchInput.value.toLowerCase().trim();
  searchResults.innerHTML = '';
  if (!q) {{ searchResults.style.display = 'none'; return; }}
  const matches = RAW_NODES.filter(n => n.label.toLowerCase().includes(q)).slice(0, 20);
  if (!matches.length) {{ searchResults.style.display = 'none'; return; }}
  searchResults.style.display = 'block';
  matches.forEach(n => {{
    const el = document.createElement('div');
    el.className = 'search-item'; el.textContent = n.label; el.style.borderLeft = `3px solid ${{n.color.background}}`; el.style.paddingLeft = '8px';
    el.onclick = () => {{
      network.focus(n.id, {{ scale: 1.5, animation: true }}); network.selectNodes([n.id]); showInfo(n.id);
      searchResults.style.display = 'none'; searchInput.value = '';
    }};
    searchResults.appendChild(el);
  }});
}});
document.addEventListener('click', e => {{
  if (!searchResults.contains(e.target) && e.target !== searchInput) searchResults.style.display = 'none';
}});

const hiddenCommunities = new Set();
const legendEl = document.getElementById('legend');
LEGEND.forEach(c => {{
  const item = document.createElement('div');
  item.className = 'legend-item';
  item.innerHTML = `<div class="legend-dot" style="background:${{c.color}}"></div>
    <span class="legend-label">${{c.label}}</span>
    <span class="legend-count">${{c.count}}</span>`;
  item.onclick = () => {{
    if (hiddenCommunities.has(c.cid)) {{ hiddenCommunities.delete(c.cid); item.classList.remove('dimmed'); }}
    else {{ hiddenCommunities.add(c.cid); item.classList.add('dimmed'); }}
    const updates = RAW_NODES.filter(n => n.community === c.cid).map(n => ({{ id: n.id, hidden: hiddenCommunities.has(c.cid) }}));
    nodesDS.update(updates);
  }};
  legendEl.appendChild(item);
}});
</script>"#, nodes_json, edges_json, legend_json)
}

fn node_community_map(communities: &HashMap<usize, Vec<String>>) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (cid, nodes) in communities { for n in nodes { map.insert(n.clone(), *cid); } }
    map
}

pub fn to_json(g: &MyGraph, communities: &HashMap<usize, Vec<String>>, output_path: &str) {
    let nc_map = node_community_map(communities);
    let mut nodes = Vec::new();
    let mut id_to_idx = HashMap::new();
    for (idx, node) in g.node_references() {
        id_to_idx.insert(node.id.clone(), idx);
        let mut n = json!({
            "id": node.id, "label": node.label, "file_type": node.file_type, "source_file": node.source_file,
            "community": nc_map.get(&node.id).copied()
        });
        if let Some(obj) = n.as_object_mut() { for (k, v) in &node.extra { obj.insert(k.clone(), v.clone()); } }
        nodes.push(n);
    }
    let mut links = Vec::new();
    for edge in g.edge_references() {
        let u = &g.node_weight(edge.source()).unwrap().id; let v = &g.node_weight(edge.target()).unwrap().id;
        let conf = &edge.weight().confidence;
        let score = edge.weight().extra.get("confidence_score").and_then(|s| s.as_f64()).unwrap_or_else(|| match conf.as_str() { "EXTRACTED" => 1.0, "INFERRED" => 0.5, "AMBIGUOUS" => 0.2, _ => 1.0 });
        let mut link = json!({ "source": u, "target": v, "relation": edge.weight().relation, "confidence": conf, "confidence_score": score });
        if let Some(obj) = link.as_object_mut() { for (k, v) in &edge.weight().extra { obj.insert(k.clone(), v.clone()); } }
        links.push(link);
    }
    let hyperedges = json!([]);
    let data = json!({ "directed": false, "multigraph": false, "graph": {}, "nodes": nodes, "links": links, "hyperedges": hyperedges });
    fs::write(output_path, serde_json::to_string_pretty(&data).unwrap()).unwrap();
}

pub fn to_cypher(g: &MyGraph, output_path: &str) {
    let mut lines = vec!["// Neo4j Cypher import - generated by /graphify".to_string(), "".to_string()];
    let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
    let re_alnum = Regex::new(r"[^A-Za-z0-9_]").unwrap();
    for (_, node) in g.node_references() {
        let label = escape(&node.label); let id_esc = escape(&node.id);
        let mut ftype = re_alnum.replace_all(&node.file_type, "").to_string();
        if ftype.is_empty() { ftype = "Entity".to_string(); } else {
            let mut c = ftype.chars(); ftype = match c.next() { None => String::new(), Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), };
        }
        lines.push(format!("MERGE (n:{} {{id: '{}', label: '{}'}});", ftype, id_esc, label));
    }
    lines.push("".to_string());
    for edge in g.edge_references() {
        let u = &g.node_weight(edge.source()).unwrap().id; let v = &g.node_weight(edge.target()).unwrap().id;
        let rel = re_alnum.replace_all(&edge.weight().relation.to_uppercase(), "_").to_string();
        let rel = if rel.is_empty() { "RELATES_TO".to_string() } else { rel };
        let conf = escape(&edge.weight().confidence);
        lines.push(format!("MATCH (a {{id: '{}'}}), (b {{id: '{}'}}) MERGE (a)-[:{} {{confidence: '{}'}}]->(b);", escape(u), escape(v), rel, conf));
    }
    fs::write(output_path, lines.join("\n")).unwrap();
}

pub fn to_graphml(g: &MyGraph, communities: &HashMap<usize, Vec<String>>, output_path: &str) {
    let nc_map = node_community_map(communities);
    let mut lines = vec![
        r#"<?xml version="1.0" encoding="utf-8"?><graphml xmlns="http://graphml.graphdrawing.org/xmlns" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">"#.to_string(),
        r#"<key id="d0" for="node" attr.name="community" attr.type="int"/>"#.to_string(),
        r#"<key id="d1" for="node" attr.name="label" attr.type="string"/>"#.to_string(),
        r#"<key id="d2" for="edge" attr.name="relation" attr.type="string"/>"#.to_string(),
        r#"<key id="d3" for="edge" attr.name="confidence" attr.type="string"/>"#.to_string(),
        r#"<graph edgedefault="undirected">"#.to_string()
    ];
    for (_, node) in g.node_references() {
        let cid = nc_map.get(&node.id).copied().unwrap_or(usize::MAX);
        lines.push(format!(r#"  <node id="{}">"#, html_escape::encode_text(&node.id)));
        if cid != usize::MAX { lines.push(format!(r#"    <data key="d0">{}</data>"#, cid)); }
        lines.push(format!(r#"    <data key="d1">{}</data>"#, html_escape::encode_text(&node.label)));
        lines.push(r#"  </node>"#.to_string());
    }
    let mut edge_idx = 0;
    for edge in g.edge_references() {
        let u = &g.node_weight(edge.source()).unwrap().id; let v = &g.node_weight(edge.target()).unwrap().id;
        lines.push(format!(r#"  <edge id="e{}" source="{}" target="{}">"#, edge_idx, html_escape::encode_text(u), html_escape::encode_text(v)));
        lines.push(format!(r#"    <data key="d2">{}</data>"#, html_escape::encode_text(&edge.weight().relation)));
        lines.push(format!(r#"    <data key="d3">{}</data>"#, html_escape::encode_text(&edge.weight().confidence)));
        lines.push(r#"  </edge>"#.to_string());
        edge_idx += 1;
    }
    lines.push(r#"</graph></graphml>"#.to_string());
    fs::write(output_path, lines.join("\n")).unwrap();
}

pub fn to_html(
    g: &MyGraph, communities: &HashMap<usize, Vec<String>>, output_path: &str, community_labels: Option<&HashMap<usize, String>>,
) -> Result<(), String> {
    if g.node_count() > MAX_NODES_FOR_VIZ { return Err(format!("Graph has {} nodes - too large for HTML viz.", g.node_count())); }
    let empty_labels = HashMap::new(); let labels = community_labels.unwrap_or(&empty_labels);
    let nc_map = node_community_map(communities);
    let mut max_deg = 1;
    for idx in g.node_indices() { max_deg = max_deg.max(g.edges(idx).count()); }
    let mut vis_nodes = Vec::new();
    let id_to_idx = |g: &MyGraph, id: &str| -> Option<petgraph::graph::NodeIndex> {
        for (idx, node) in g.node_references() { if node.id == id { return Some(idx); } }
        None
    };
    for (_, node) in g.node_references() {
        let cid = nc_map.get(&node.id).copied().unwrap_or(0);
        let color = COMMUNITY_COLORS[cid % COMMUNITY_COLORS.len()];
        let label = sanitize_label(&if node.label.is_empty() { node.id.clone() } else { node.label.clone() });
        let deg = g.edges(id_to_idx(g, &node.id).unwrap()).count();
        let size = 10.0 + 30.0 * (deg as f64 / max_deg as f64);
        let font_size = if deg as f64 >= max_deg as f64 * 0.15 { 12 } else { 0 };
        vis_nodes.push(json!({
            "id": node.id, "label": label, "color": { "background": color, "border": color, "highlight": { "background": "#ffffff", "border": color } },
            "size": size, "font": { "size": font_size, "color": "#ffffff" }, "title": label, "community": cid,
            "community_name": sanitize_label(&labels.get(&cid).cloned().unwrap_or_else(|| format!("Community {}", cid))),
            "source_file": sanitize_label(&node.source_file), "file_type": node.file_type, "degree": deg
        }));
    }
    let mut vis_edges = Vec::new();
    for edge in g.edge_references() {
        let conf = &edge.weight().confidence; let rel = &edge.weight().relation; let is_ext = conf == "EXTRACTED";
        vis_edges.push(json!({
            "from": g.node_weight(edge.source()).unwrap().id, "to": g.node_weight(edge.target()).unwrap().id, "label": rel, "title": format!("{} [{}]", rel, conf),
            "dashes": !is_ext, "width": if is_ext { 2 } else { 1 }, "color": { "opacity": if is_ext { 0.7 } else { 0.35 } }, "confidence": conf
        }));
    }
    let mut legend_data = Vec::new();
    let mut sorted_cids: Vec<_> = labels.keys().copied().collect(); sorted_cids.sort();
    if sorted_cids.is_empty() { let mut ks: Vec<_> = communities.keys().copied().collect(); ks.sort(); sorted_cids = ks; }
    for cid in sorted_cids {
        let color = COMMUNITY_COLORS[cid % COMMUNITY_COLORS.len()];
        let lbl = labels.get(&cid).cloned().unwrap_or_else(|| format!("Community {}", cid));
        let count = communities.get(&cid).map(|v| v.len()).unwrap_or(0);
        legend_data.push(json!({ "cid": cid, "color": color, "label": lbl, "count": count }));
    }
    let nodes_json = serde_json::to_string(&vis_nodes).unwrap(); let edges_json = serde_json::to_string(&vis_edges).unwrap(); let legend_json = serde_json::to_string(&legend_data).unwrap();
    let hyperedges_json = "[]";
    let sanitized_title = sanitize_label(output_path); let title = html_escape::encode_text(&sanitized_title);
    let stats = format!("{} nodes &middot; {} edges &middot; {} communities", g.node_count(), g.edge_count(), communities.len());
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>graphify - {}</title><script src="https://unpkg.com/vis-network/standalone/umd/vis-network.min.js"></script>{}</head>
<body><div id="graph"></div><div id="sidebar"><div id="search-wrap"><input id="search" type="text" placeholder="Search nodes..." autocomplete="off"><div id="search-results"></div></div><div id="info-panel"><h3>Node Info</h3><div id="info-content"><span class="empty">Click a node to inspect it</span></div></div><div id="legend-wrap"><h3>Communities</h3><div id="legend"></div></div><div id="stats">{}</div></div>{}{}</script></body></html>"#, title, html_styles(), stats, html_script(&nodes_json, &edges_json, &legend_json), hyperedge_script(hyperedges_json));
    fs::write(output_path, html).unwrap(); Ok(())
}
