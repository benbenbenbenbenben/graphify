use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const HOOK_MARKER: &str = "# graphify-hook-start";
pub const HOOK_MARKER_END: &str = "# graphify-hook-end";
pub const CHECKOUT_MARKER: &str = "# graphify-checkout-hook-start";
pub const CHECKOUT_MARKER_END: &str = "# graphify-checkout-hook-end";

const HOOK_SCRIPT: &str = r#"# graphify-hook-start
CHANGED=$(git diff --name-only HEAD~1 HEAD 2>/dev/null || git diff --name-only HEAD 2>/dev/null)
if [ -z "$CHANGED" ]; then
    return 0 2>/dev/null
fi
export GRAPHIFY_CHANGED="$CHANGED"
python3 -c "
import os, sys
from pathlib import Path
CODE_EXTS = {'.py', '.ts', '.js', '.go', '.rs', '.java', '.cpp', '.c', '.rb', '.swift', '.kt', '.cs', '.scala', '.php', '.cc', '.cxx', '.hpp', '.h', '.kts'}
changed = [Path(f.strip()) for f in os.environ.get('GRAPHIFY_CHANGED', '').strip().splitlines() if f.strip()]
code_changed = [f for f in changed if f.suffix.lower() in CODE_EXTS and f.exists()]
if not code_changed: sys.exit(0)
try:
    from graphify.watch import _rebuild_code
    _rebuild_code(Path('.'))
except Exception as exc:
    sys.exit(1)
"
# graphify-hook-end"#;

const CHECKOUT_SCRIPT: &str = r#"# graphify-checkout-hook-start
if [ "$3" != "1" ] || [ ! -d "graphify-out" ]; then return 0 2>/dev/null; fi
python3 -c "
from graphify.watch import _rebuild_code
from pathlib import Path
import sys
try:
    _rebuild_code(Path('.'))
except Exception as exc:
    sys.exit(1)
"
# graphify-checkout-hook-end"#;

fn git_root(path: &Path) -> Option<PathBuf> {
    let current = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if current.join(".git").exists() { return Some(current); }
    for parent in current.ancestors() {
        if parent.join(".git").exists() { return Some(parent.to_path_buf()); }
    }
    None
}

fn install_hook(hooks_dir: &Path, name: &str, script: &str, marker: &str) -> String {
    let hook_path = hooks_dir.join(name);
    if hook_path.exists() {
        if let Ok(content) = fs::read_to_string(&hook_path) {
            if content.contains(marker) { return format!("already installed at {}", hook_path.display()); }
            let _ = fs::write(&hook_path, format!("{}\n\n{}", content.trim_end(), script));
            return format!("appended to existing {} hook at {}", name, hook_path.display());
        }
    }
    let _ = fs::write(&hook_path, format!("#!/bin/bash\n{}", script));
    let mut perms = fs::metadata(&hook_path).unwrap().permissions();
    perms.set_mode(0o755);
    let _ = fs::set_permissions(&hook_path, perms);
    format!("installed at {}", hook_path.display())
}

fn uninstall_hook(hooks_dir: &Path, name: &str, marker: &str, marker_end: &str) -> String {
    let hook_path = hooks_dir.join(name);
    if !hook_path.exists() { return format!("no {} hook found - nothing to remove.", name); }
    let content = fs::read_to_string(&hook_path).unwrap_or_default();
    if !content.contains(marker) { return format!("graphify hook not found in {} - nothing to remove.", name); }
    let re_str = format!(r"(?s){}.*?{}\n?", regex::escape(marker), regex::escape(marker_end));
    let re = regex::Regex::new(&re_str).unwrap();
    let new_content = re.replace_all(&content, "").trim().to_string();
    if new_content.is_empty() || new_content == "#!/bin/bash" {
        let _ = fs::remove_file(&hook_path);
        return format!("removed {} hook at {}", name, hook_path.display());
    }
    let _ = fs::write(&hook_path, format!("{}\n", new_content));
    format!("graphify removed from {} at {} (other hook content preserved)", name, hook_path.display())
}

pub fn install(path: &Path) -> Result<String, String> {
    if let Some(root) = git_root(path) {
        let hooks_dir = root.join(".git").join("hooks");
        let _ = fs::create_dir_all(&hooks_dir);
        let c_msg = install_hook(&hooks_dir, "post-commit", HOOK_SCRIPT, HOOK_MARKER);
        let co_msg = install_hook(&hooks_dir, "post-checkout", CHECKOUT_SCRIPT, CHECKOUT_MARKER);
        Ok(format!("post-commit: {}\npost-checkout: {}", c_msg, co_msg))
    } else { Err(format!("No git repository found at or above {}", path.display())) }
}

pub fn uninstall(path: &Path) -> Result<String, String> {
    if let Some(root) = git_root(path) {
        let hooks_dir = root.join(".git").join("hooks");
        let c_msg = uninstall_hook(&hooks_dir, "post-commit", HOOK_MARKER, HOOK_MARKER_END);
        let co_msg = uninstall_hook(&hooks_dir, "post-checkout", CHECKOUT_MARKER, CHECKOUT_MARKER_END);
        Ok(format!("post-commit: {}\npost-checkout: {}", c_msg, co_msg))
    } else { Err(format!("No git repository found at or above {}", path.display())) }
}

pub fn status(path: &Path) -> String {
    if let Some(root) = git_root(path) {
        let hooks_dir = root.join(".git").join("hooks");
        let check = |n: &str, m: &str| -> String {
            let p = hooks_dir.join(n);
            if !p.exists() { "not installed".to_string() }
            else if fs::read_to_string(p).unwrap_or_default().contains(m) { "installed".to_string() }
            else { "not installed (hook exists but graphify not found)".to_string() }
        };
        format!("post-commit: {}\npost-checkout: {}", check("post-commit", HOOK_MARKER), check("post-checkout", CHECKOUT_MARKER))
    } else { "Not in a git repository.".to_string() }
}
