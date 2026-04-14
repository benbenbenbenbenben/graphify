pub mod validate;
pub mod build;
pub mod cache;
pub mod detect;
pub mod manifest;
pub mod security;
pub mod cluster;
pub mod analyze;
pub mod report;
pub mod ingest;
pub mod watch;
pub mod wiki;
pub mod hooks;
pub mod export;
pub mod extract;
pub mod serve;
pub mod benchmark;

#[cfg(test)]
#[path = "validate_test.rs"]
mod validate_test;

#[cfg(test)]
#[path = "build_test.rs"]
mod build_test;

#[cfg(test)]
#[path = "cache_test.rs"]
mod cache_test;

#[cfg(test)]
#[path = "detect_test.rs"]
mod detect_test;

#[cfg(test)]
#[path = "security_test.rs"]
mod security_test;

#[cfg(test)]
#[path = "cluster_test.rs"]
mod cluster_test;

#[cfg(test)]
#[path = "analyze_test.rs"]
mod analyze_test;

#[cfg(test)]
#[path = "report_test.rs"]
mod report_test;

#[cfg(test)]
#[path = "ingest_test.rs"]
mod ingest_test;

#[cfg(test)]
#[path = "watch_test.rs"]
mod watch_test;

#[cfg(test)]
#[path = "wiki_test.rs"]
mod wiki_test;

#[cfg(test)]
#[path = "hooks_test.rs"]
mod hooks_test;

#[cfg(test)]
#[path = "export_test.rs"]
mod export_test;

#[cfg(test)]
#[path = "extract_test.rs"]
mod extract_test;

#[cfg(test)]
#[path = "serve_test.rs"]
mod serve_test;

#[cfg(test)]
#[path = "benchmark_test.rs"]
mod benchmark_test;
