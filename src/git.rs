use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_site_root(published: &Path) -> PathBuf {
    for anc in published.ancestors() {
        if let Some(name) = anc.file_name().and_then(|s| s.to_str())
            && name == "content"
        {
            return anc.parent().unwrap().to_path_buf();
        }
    }
    published.parent().unwrap().to_path_buf()
}

fn rel_args(site_root: &Path, paths: &[PathBuf]) -> Vec<OsString> {
    paths
        .iter()
        .map(|p| {
            p.strip_prefix(site_root)
                .map(|rel| rel.as_os_str().to_os_string())
                .unwrap_or_else(|_| p.as_os_str().to_os_string())
        })
        .collect()
}

fn reset_paths(site_root: &Path, paths: &[OsString]) {
    let _ = Command::new("git")
        .arg("reset")
        .arg("HEAD")
        .args(paths)
        .current_dir(site_root)
        .status();
}

pub fn run_git_steps(site_root: &Path, commit_msg: &str, paths: &[PathBuf]) -> Result<()> {
    // Ensure it's a git repo
    let git_check = Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .current_dir(site_root)
        .output()?;
    if !git_check.status.success() {
        return Err(anyhow::anyhow!(
            "Directory {} is not a git repository. git rev-parse failed: {}",
            site_root.display(),
            String::from_utf8_lossy(&git_check.stderr)
        ));
    }

    // Avoid mixing with pre-staged changes
    let pre_staged = Command::new("git")
        .arg("diff")
        .arg("--cached")
        .arg("--quiet")
        .current_dir(site_root)
        .status()?;
    if !pre_staged.success() {
        return Err(anyhow::anyhow!(
            "Repository {} has pre-existing staged changes; commit or reset them before running nuch.",
            site_root.display()
        ));
    }

    let rels = rel_args(site_root, paths);

    let git_add = Command::new("git")
        .arg("add")
        .args(&rels)
        .current_dir(site_root)
        .output()?;
    if !git_add.status.success() {
        return Err(anyhow::anyhow!(
            "git add failed: {}",
            String::from_utf8_lossy(&git_add.stderr)
        ));
    }

    let git_commit = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(commit_msg)
        .current_dir(site_root)
        .output()?;
    if !git_commit.status.success() {
        reset_paths(site_root, &rels);
        return Err(anyhow::anyhow!(
            "git commit failed: {}",
            String::from_utf8_lossy(&git_commit.stderr)
        ));
    }

    let git_push = Command::new("git")
        .arg("push")
        .current_dir(site_root)
        .output()?;
    if !git_push.status.success() {
        reset_paths(site_root, &rels);
        return Err(anyhow::anyhow!(
            "git push failed: {}",
            String::from_utf8_lossy(&git_push.stderr)
        ));
    }

    Ok(())
}
