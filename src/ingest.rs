use crate::security::{safe_fetch, safe_fetch_text, validate_url, SecurityError, MAX_FETCH_BYTES, MAX_TEXT_BYTES};
use chrono::Utc;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

fn yaml_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ").replace('\r', " ")
}

fn safe_filename(url_str: &str, suffix: &str) -> String {
    if let Ok(parsed) = Url::parse(url_str) {
        let name = format!("{}{}", parsed.host_str().unwrap_or(""), parsed.path());
        let re = Regex::new(r"[^\w\-]").unwrap();
        let mut clean = re.replace_all(&name, "_").into_owned();
        clean = clean.trim_matches('_').to_string();
        let re_mult = Regex::new(r"_+").unwrap();
        clean = re_mult.replace_all(&clean, "_").into_owned();
        let limit = 80.min(clean.len());
        format!("{}{}", &clean[..limit], suffix)
    } else {
        format!("unknown{}", suffix)
    }
}

fn detect_url_type(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    if lower.contains("twitter.com") || lower.contains("x.com") { return "tweet"; }
    if lower.contains("arxiv.org") { return "arxiv"; }
    if lower.contains("github.com") { return "github"; }
    if lower.contains("youtube.com") || lower.contains("youtu.be") { return "youtube"; }
    if let Ok(parsed) = Url::parse(url) {
        let path = parsed.path().to_lowercase();
        if path.ends_with(".pdf") { return "pdf"; }
        if [".png", ".jpg", ".jpeg", ".webp", ".gif"].iter().any(|ext| path.ends_with(ext)) { return "image"; }
    }
    "webpage"
}

fn html_to_markdown(html: &str, _url: &str) -> String {
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let re_tag = Regex::new(r"(?is)<[^>]+>").unwrap();
    let re_space = Regex::new(r"(?is)\s+").unwrap();

    let text = re_script.replace_all(html, "");
    let text = re_style.replace_all(&text, "");
    let text = re_tag.replace_all(&text, " ");
    let text = re_space.replace_all(&text, " ").trim().to_string();
    let limit = 8000.min(text.len());
    text[..limit].to_string()
}

fn fetch_tweet(url: &str, author: Option<&str>, contributor: Option<&str>) -> Result<(String, String), SecurityError> {
    let oembed_url = url.replace("x.com", "twitter.com");
    let encoded = urlencoding::encode(&oembed_url);
    let oembed_api = format!("https://publish.twitter.com/oembed?url={}&omit_script=true", encoded);

    let (tweet_text, tweet_author) = match safe_fetch_text(&oembed_api, MAX_TEXT_BYTES) {
        Ok(text) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let html = v.get("html").and_then(|h| h.as_str()).unwrap_or("");
                let re_tag = Regex::new(r"(?is)<[^>]+>").unwrap();
                let clean = re_tag.replace_all(html, "").trim().to_string();
                let auth = v.get("author_name").and_then(|a| a.as_str()).unwrap_or("unknown").to_string();
                (clean, auth)
            } else { (format!("Tweet at {} (could not parse JSON)", url), "unknown".to_string()) }
        },
        Err(_) => { (format!("Tweet at {} (could not fetch content)", url), "unknown".to_string()) }
    };

    let now = Utc::now().to_rfc3339();
    let contrib = contributor.or(author).unwrap_or("unknown");

    let content = format!(
"---
source_url: {}
type: tweet
author: {}
captured_at: {}
contributor: {}
---

# Tweet by @{}

{}

Source: {}
", url, tweet_author, now, contrib, tweet_author, tweet_text, url);
    let filename = safe_filename(url, ".md");
    Ok((content, filename))
}

fn fetch_webpage(url: &str, author: Option<&str>, contributor: Option<&str>) -> Result<(String, String), SecurityError> {
    let html = safe_fetch_text(url, MAX_TEXT_BYTES)?;
    let re_title = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    let title = if let Some(caps) = re_title.captures(&html) {
        let re_space = Regex::new(r"(?is)\s+").unwrap();
        re_space.replace_all(&caps[1], " ").trim().to_string()
    } else { url.to_string() };

    let markdown = html_to_markdown(&html, url);
    let now = Utc::now().to_rfc3339();
    let contrib = contributor.or(author).unwrap_or("unknown");

    let limit = 12000.min(markdown.len());
    let content = format!(
"---
source_url: {}
type: webpage
title: \"{}\"
captured_at: {}
contributor: {}
---

# {}

Source: {}

---

{}
", url, yaml_str(&title), now, contrib, title, url, &markdown[..limit]);

    let filename = safe_filename(url, ".md");
    Ok((content, filename))
}

fn fetch_arxiv(url: &str, author: Option<&str>, contributor: Option<&str>) -> Result<(String, String), SecurityError> {
    let re_id = Regex::new(r"(\d{4}\.\d{4,5})").unwrap();
    if let Some(caps) = re_id.captures(url) {
        let arxiv_id = &caps[1];
        let api_url = format!("https://export.arxiv.org/abs/{}", arxiv_id);

        let (title, abstract_text, paper_authors) = match safe_fetch_text(&api_url, MAX_TEXT_BYTES) {
            Ok(html) => {
                let re_abs = Regex::new(r"(?is)class=.abstract[^>]*>(.*?)</blockquote>").unwrap();
                let re_title = Regex::new(r"(?is)class=.title[^>]*>(.*?)</h1>").unwrap();
                let re_authors = Regex::new(r"(?is)class=.authors.[^>]*>(.*?)</div>").unwrap();
                let re_tag = Regex::new(r"(?is)<[^>]+>").unwrap();

                let a = re_abs.captures(&html).map(|c| re_tag.replace_all(&c[1], "").trim().to_string()).unwrap_or_default();
                let t = re_title.captures(&html).map(|c| re_tag.replace_all(&c[1], " ").trim().to_string()).unwrap_or_else(|| arxiv_id.to_string());
                let auth = re_authors.captures(&html).map(|c| re_tag.replace_all(&c[1], "").trim().to_string()).unwrap_or_default();
                (t, a, auth)
            },
            Err(_) => (arxiv_id.to_string(), "".to_string(), "".to_string())
        };

        let now = Utc::now().to_rfc3339();
        let contrib = contributor.or(author).unwrap_or("unknown");
        let content = format!(
"---
source_url: {}
arxiv_id: {}
type: paper
title: \"{}\"
paper_authors: \"{}\"
captured_at: {}
contributor: {}
---

# {}

**Authors:** {}
**arXiv:** {}

## Abstract

{}

Source: {}
", url, arxiv_id, yaml_str(&title), yaml_str(&paper_authors), now, contrib, title, paper_authors, arxiv_id, abstract_text, url);

        let filename = format!("arxiv_{}.md", arxiv_id.replace('.', "_"));
        Ok((content, filename))
    } else {
        fetch_webpage(url, author, contributor)
    }
}

fn download_binary(url: &str, suffix: &str, target_dir: &Path) -> Result<PathBuf, SecurityError> {
    let filename = safe_filename(url, suffix);
    let out_path = target_dir.join(filename);
    let bytes = safe_fetch(url, MAX_FETCH_BYTES)?;
    fs::write(&out_path, bytes)?;
    Ok(out_path)
}

pub fn ingest(url: &str, target_dir: &Path, author: Option<&str>, contributor: Option<&str>) -> Result<PathBuf, SecurityError> {
    fs::create_dir_all(target_dir)?;
    validate_url(url)?;
    let url_type = detect_url_type(url);
    if url_type == "pdf" { return download_binary(url, ".pdf", target_dir); }
    if url_type == "image" {
        let parsed = Url::parse(url)?;
        let path = Path::new(parsed.path());
        let suffix = path.extension().and_then(|s| s.to_str()).map(|s| format!(".{}", s)).unwrap_or_else(|| ".jpg".to_string());
        return download_binary(url, &suffix, target_dir);
    }

    let (content, mut filename) = match url_type {
        "tweet" => fetch_tweet(url, author, contributor)?,
        "arxiv" => fetch_arxiv(url, author, contributor)?,
        _ => fetch_webpage(url, author, contributor)?,
    };

    let mut out_path = target_dir.join(&filename);
    let mut counter = 1;
    let stem = Path::new(&filename).file_stem().unwrap().to_string_lossy().into_owned();
    while out_path.exists() {
        filename = format!("{}_{}.md", stem, counter);
        out_path = target_dir.join(&filename);
        counter += 1;
    }

    fs::write(&out_path, content)?;
    Ok(out_path)
}

pub fn save_query_result(
    question: &str, answer: &str, memory_dir: &Path, query_type: &str, source_nodes: Option<&[String]>
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(memory_dir)?;
    let now = Utc::now();
    let re = Regex::new(r"[^\w]").unwrap();
    let slug_full = re.replace_all(&question.to_lowercase(), "_").into_owned();
    let slug_trimmed = slug_full.trim_matches('_');
    let limit = 50.min(slug_trimmed.len());
    let slug = &slug_trimmed[..limit];
    let filename = format!("query_{}_{}.md", now.format("%Y%m%d_%H%M%S"), slug);

    let mut frontmatter_lines = vec![
        "---".to_string(),
        format!("type: \"{}\"", query_type),
        format!("date: \"{}\"", now.to_rfc3339()),
        format!("question: \"{}\"", yaml_str(question)),
        "contributor: \"graphify\"".to_string(),
    ];
    if let Some(nodes) = source_nodes {
        let quoted: Vec<String> = nodes.iter().take(10).map(|n| format!("\"{}\"", n)).collect();
        frontmatter_lines.push(format!("source_nodes: [{}]", quoted.join(", ")));
    }
    frontmatter_lines.push("---".to_string());

    let mut body_lines = vec![
        "".to_string(), format!("# Q: {}", question), "".to_string(),
        "## Answer".to_string(), "".to_string(), answer.to_string(),
    ];
    if let Some(nodes) = source_nodes {
        body_lines.push("".to_string());
        body_lines.push("## Source Nodes".to_string());
        body_lines.push("".to_string());
        for n in nodes { body_lines.push(format!("- {}", n)); }
    }

    let mut content = frontmatter_lines.join("\n");
    content.push('\n');
    content.push_str(&body_lines.join("\n"));
    let out_path = memory_dir.join(filename);
    fs::write(&out_path, content)?;
    Ok(out_path)
}
