use crate::hooks::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn make_git_repo(dir: &TempDir) -> PathBuf {
    Command::new("git").args(&["init", dir.path().to_str().unwrap()]).output().unwrap();
    dir.path().to_path_buf()
}

#[test] fn test_install_creates_hook() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); let res = install(&repo).unwrap(); assert!(repo.join(".git/hooks/post-commit").exists()); assert!(res.contains("installed")); }
#[test] fn test_install_idempotent() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); install(&repo).unwrap(); let res = install(&repo).unwrap(); assert!(res.contains("already installed")); }
#[test] fn test_install_appends_to_existing_hook() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); let hook = repo.join(".git/hooks/post-commit"); fs::create_dir_all(hook.parent().unwrap()).unwrap(); fs::write(&hook, "#!/bin/bash\necho existing\n").unwrap(); install(&repo).unwrap(); assert!(fs::read_to_string(&hook).unwrap().contains("existing")); }
#[test] fn test_uninstall_removes_hook() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); install(&repo).unwrap(); uninstall(&repo).unwrap(); assert!(!repo.join(".git/hooks/post-commit").exists()); }
#[test] fn test_uninstall_no_hook() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); assert!(uninstall(&repo).unwrap().contains("nothing to remove")); }
#[test] fn test_status_installed() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); install(&repo).unwrap(); assert!(status(&repo).contains("installed")); }
#[test] fn test_status_not_installed() { let dir = TempDir::new().unwrap(); let repo = make_git_repo(&dir); assert!(status(&repo).contains("not installed")); }
#[test] fn test_no_git_repo_raises() { let dir = TempDir::new().unwrap(); assert!(install(&dir.path().join("not_a_repo")).is_err()); }
