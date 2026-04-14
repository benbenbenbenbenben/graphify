use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

lazy_static::lazy_static! {
    pub static ref WATCHED_EXTENSIONS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        for ext in &["py", "ts", "js", "go", "rs", "java", "cpp", "c", "rb", "swift", "kt", "cs", "scala", "php", "cc", "cxx", "hpp", "h", "kts", "md", "txt", "rst", "pdf", "png", "jpg", "jpeg", "webp", "gif", "svg"] { s.insert(*ext); }
        s
    };
    pub static ref CODE_EXTENSIONS: HashSet<&'static str> = {
        let mut s = HashSet::new();
        for ext in &["py", "ts", "js", "go", "rs", "java", "cpp", "c", "rb", "swift", "kt", "cs", "scala", "php", "cc", "cxx", "hpp", "h", "kts"] { s.insert(*ext); }
        s
    };
}

pub fn notify_only(watch_path: &Path) {
    let flag = watch_path.join("graphify-out").join("needs_update");
    if let Some(parent) = flag.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(&flag, "1");
    println!("\n[graphify watch] New or changed files detected in {:?}", watch_path);
}

pub fn has_non_code(changed_paths: &[PathBuf]) -> bool {
    for p in changed_paths {
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            if !CODE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) { return true; }
        } else { return true; }
    }
    false
}

#[cfg(feature = "watch")]
pub fn watch(watch_path: &Path, debounce: f64) -> notify::Result<()> {
    use notify::{Watcher, RecursiveMode, EventKind};
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(watch_path, RecursiveMode::Recursive)?;
    println!("[graphify watch] Watching {:?}", watch_path);
    let debounce_dur = Duration::from_secs_f64(debounce);
    let mut last_trigger = Instant::now();
    let mut pending = false;
    let mut changed: HashSet<PathBuf> = HashSet::new();
    loop {
        while let Ok(Ok(event)) = rx.recv_timeout(Duration::from_millis(100)) {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if !WATCHED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) { continue; }
                        } else { continue; }
                        if path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) { continue; }
                        if path.components().any(|c| c.as_os_str() == "graphify-out") { continue; }
                        last_trigger = Instant::now(); pending = true; changed.insert(path);
                    }
                }, _ => {}
            }
        }
        if pending && last_trigger.elapsed() >= debounce_dur {
            pending = false;
            let batch: Vec<PathBuf> = changed.drain().collect();
            if has_non_code(&batch) { notify_only(watch_path); } else { println!("[graphify watch] Code rebuild logic."); }
        }
    }
}
