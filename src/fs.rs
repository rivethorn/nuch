use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn dir_has_markdown(dir: &std::path::Path) -> Result<bool, std::io::Error> {
    if !dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn matching_images_for_stem(
    stem_lower: &str,
    dir: &Path,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut images = Vec::new();
    if !dir.is_dir() {
        return Ok(images);
    }
    let exts = ["png", "jpg", "jpeg", "gif", "webp", "svg"];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
            let name_lower = name.to_lowercase();
            if !name_lower.starts_with(stem_lower) {
                continue;
            }
            if exts.iter().any(|ext| name_lower.ends_with(ext)) {
                images.push(p);
            }
        }
    }
    Ok(images)
}

pub fn copy_file_to(src: &PathBuf, dst_dir: &PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(dst_dir)?;
    let dst = dst_dir.join(src.file_name().unwrap());
    if dst.exists() {
        return Err(anyhow::anyhow!(
            "Destination already exists: {}",
            dst.display()
        ));
    }
    fs::copy(src, &dst).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy {} to {}: {}",
            src.display(),
            dst.display(),
            e
        )
    })?;
    Ok(dst)
}

#[allow(dead_code)]
pub fn remove_files(paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        if p.exists() {
            fs::remove_file(p)
                .map_err(|e| anyhow::anyhow!("Failed to remove {}: {}", p.display(), e))?;
        }
    }
    Ok(())
}

pub fn rollback_remove_files(created: &[PathBuf]) -> Vec<String> {
    let mut failures = Vec::new();
    for f in created {
        if f.exists()
            && let Err(e) = fs::remove_file(f)
        {
            failures.push(format!("Failed to remove {}: {}", f.display(), e));
        }
    }
    failures
}

pub fn backup_files_to_temp(files: &[PathBuf]) -> Result<(PathBuf, Vec<(PathBuf, PathBuf)>)> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("nuch-delete-{}", ts));
    fs::create_dir_all(&tmp)?;

    let mut pairs = Vec::new();
    for orig in files {
        if !orig.exists() {
            continue;
        }
        let dest = tmp.join(orig.file_name().unwrap());
        fs::copy(orig, &dest)
            .map_err(|e| anyhow::anyhow!("Failed to backup {}: {}", orig.display(), e))?;
        pairs.push((orig.clone(), dest));
    }

    Ok((tmp, pairs))
}

pub fn restore_from_backups(pairs: &[(PathBuf, PathBuf)]) -> Result<()> {
    for (orig, backup) in pairs {
        if backup.exists() {
            fs::copy(backup, orig)
                .map_err(|e| anyhow::anyhow!("Failed to restore {}: {}", orig.display(), e))?;
        }
    }
    Ok(())
}

pub fn cleanup_backup_dir(dir: &PathBuf) {
    if dir.exists() && dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn dir_has_markdown_detects_markdown() {
        let td = tempdir().unwrap();
        assert!(!dir_has_markdown(td.path()).unwrap());

        let md = td.path().join("post.md");
        let mut f = File::create(&md).unwrap();
        writeln!(f, "# hello").unwrap();
        assert!(dir_has_markdown(td.path()).unwrap());
    }

    #[test]
    fn matching_images_for_stem_filters_correctly() {
        let td = tempdir().unwrap();
        let files = ["post1.png", "post1-thumb.JPG", "other.png", "post1.txt"];
        for name in files.iter() {
            let p = td.path().join(name);
            std::fs::write(&p, b"data").unwrap();
        }

        let mut matches = matching_images_for_stem("post1", td.path()).unwrap();
        matches.sort();
        assert_eq!(matches.len(), 2);
        let names: Vec<_> = matches
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_lowercase())
            .collect();
        assert!(names.contains(&"post1.png".to_string()));
        assert!(names.contains(&"post1-thumb.jpg".to_string()));
    }
}
