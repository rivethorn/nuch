use anyhow::Result;
use inquire::Confirm;
use std::fs;
use std::path::PathBuf;

use crate::config::CollectionPaths;
use crate::fs as fs_helpers;
use crate::git as git_helpers;

pub fn publish_selected(
    selected: PathBuf,
    dest_path: CollectionPaths,
    working_images: Option<PathBuf>,
) -> Result<()> {
    let filename = selected
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Destination markdown path
    let dest_md = dest_path.files.join(filename);
    if dest_md.exists() {
        return Err(anyhow::anyhow!(
            "Destination markdown already exists: {}",
            dest_md.display()
        ));
    }

    // Copy markdown
    fs::create_dir_all(&dest_path.files)?;
    fs::copy(&selected, &dest_md)
        .map_err(|e| anyhow::anyhow!("Failed to copy markdown to {}: {}", dest_md.display(), e))?;

    // Keep track of created files for rollback
    let mut created: Vec<PathBuf> = vec![dest_md.clone()];

    // Copy images if configured
    if let (Some(src_images), Some(dst_images)) = (working_images, dest_path.images) {
        let stem = selected
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename stem"))?;
        let stem_lower = stem.to_lowercase();
        let images = fs_helpers::matching_images_for_stem(&stem_lower, &src_images)?;
        if images.is_empty() {
            println!(
                "No images matching '{}' found in {}",
                stem,
                src_images.display()
            );
        } else {
            fs::create_dir_all(&dst_images)?;
            for p in images {
                let dest_img = dst_images.join(p.file_name().unwrap());
                if dest_img.exists() {
                    let failures = fs_helpers::rollback_remove_files(&created);
                    if failures.is_empty() {
                        return Err(anyhow::anyhow!(
                            "Image already exists at destination {} — aborting",
                            dest_img.display()
                        ));
                    } else {
                        return Err(anyhow::anyhow!(
                            "Image exists and rollback failures: {}",
                            failures.join("; ")
                        ));
                    }
                }
                fs::copy(&p, &dest_img).map_err(|e| {
                    let failures = fs_helpers::rollback_remove_files(&created);
                    if failures.is_empty() {
                        anyhow::anyhow!("Failed to copy image {}: {}", p.display(), e)
                    } else {
                        anyhow::anyhow!(
                            "Failed to copy image {}; rollback failures: {}",
                            p.display(),
                            failures.join("; ")
                        )
                    }
                })?;
                created.push(dest_img);
            }
        }
    }

    // Show summary and ask for confirmation
    println!("About to commit the following files:");
    for f in &created {
        println!("  {}", f.display());
    }

    if !Confirm::new("Proceed to run git add/commit/push?")
        .with_default(true)
        .prompt()?
    {
        let failures = fs_helpers::rollback_remove_files(&created);
        if failures.is_empty() {
            println!("Aborted by user; rolled back created files.");
            return Ok(());
        } else {
            return Err(anyhow::anyhow!(
                "Aborted by user; rollback failures: {}",
                failures.join("; ")
            ));
        }
    }

    let site_root = git_helpers::get_site_root(&dest_path.files);
    if let Err(e) =
        git_helpers::run_git_steps(&site_root, &format!("Add {} to blog", filename), &created)
    {
        let failures = fs_helpers::rollback_remove_files(&created);
        if failures.is_empty() {
            return Err(e);
        } else {
            return Err(anyhow::anyhow!(
                "{}; rollback failures: {}",
                e,
                failures.join("; ")
            ));
        }
    }

    println!("Published {} successfully", filename);
    Ok(())
}

pub fn delete_selected(
    selected: PathBuf,
    path: CollectionPaths,
    backup_dir: PathBuf,
    working_images: Option<PathBuf>,
) -> Result<()> {
    let filename = selected
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    let stem = selected
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename stem"))?;
    let stem_lower = stem.to_lowercase();

    // Check if markdown exists in working dir
    let working_md = backup_dir.join(filename);

    let mut backup_files: Vec<PathBuf> = Vec::new();

    if !working_md.exists() {
        let ask = format!(
            "'{}' not found in working dir. Create backup in working dir?",
            filename
        );
        if Confirm::new(&ask).with_default(true).prompt()? {
            let copied = fs_helpers::copy_file_to(&selected.to_path_buf(), &backup_dir)?;
            backup_files.push(copied.clone());

            if let (Some(pub_imgs), Some(work_imgs)) = (&path.images, &working_images) {
                let images = fs_helpers::matching_images_for_stem(&stem_lower, pub_imgs)?;
                if !images.is_empty() {
                    fs::create_dir_all(work_imgs)?;
                    for img in images.iter() {
                        let dest = work_imgs.join(img.file_name().unwrap());
                        if dest.exists() {
                            for f in &backup_files {
                                let _ = fs::remove_file(f);
                            }
                            return Err(anyhow::anyhow!(
                                "Backup target already exists: {}",
                                dest.display()
                            ));
                        }
                        fs::copy(img, &dest).map_err(|e| {
                            for f in &backup_files {
                                let _ = fs::remove_file(f);
                            }
                            anyhow::anyhow!(
                                "Failed to copy image {} to {}: {}",
                                img.display(),
                                dest.display(),
                                e
                            )
                        })?;
                        backup_files.push(dest);
                    }
                }
            }

            println!("Backup created in {}", backup_dir.display());
        } else {
            println!("Proceeding without backup.");
        }
    } else {
        println!("File exists in working dir; skipping backup.");
    }

    // Gather list of images to delete in publishing_images
    let mut to_delete: Vec<PathBuf> = Vec::new();
    to_delete.push(selected.to_path_buf());
    if let Some(pub_imgs) = &path.images {
        let images = fs_helpers::matching_images_for_stem(&stem_lower, pub_imgs)?;
        for img in images {
            to_delete.push(img);
        }
    }

    let (backup_dir, backups) = fs_helpers::backup_files_to_temp(&to_delete)?;

    println!("About to delete the following files:");
    for p in &to_delete {
        println!("  {}", p.display());
    }
    println!("Backups created at: {}", backup_dir.display());

    // Ask for confirmation
    if !Confirm::new("Proceed with deletion and git steps?")
        .with_default(true)
        .prompt()?
    {
        cleanup_and_abort(&backup_dir, &backups)?;
        println!("Aborted by user; backups at {}", backup_dir.display());
        return Ok(());
    }

    // Delete files
    for p in &to_delete {
        if p.exists()
            && let Err(e) = fs::remove_file(p)
        {
            restore_and_cleanup(&backups, &backup_dir)?;
            return Err(anyhow::anyhow!("Failed to remove {}: {}", p.display(), e));
        }
    }

    // Run git steps
    let site_root = git_helpers::get_site_root(&path.files);
    if let Err(e) = git_helpers::run_git_steps(
        &site_root,
        &format!("Remove {} from blog", filename),
        &to_delete,
    ) {
        if let Err(rest_err) = fs_helpers::restore_from_backups(&backups) {
            eprintln!("Failed to restore from backups: {}", rest_err);
        }
        fs_helpers::cleanup_backup_dir(&backup_dir);
        return Err(e);
    }

    fs_helpers::cleanup_backup_dir(&backup_dir);
    println!("Deleted {} and corresponding images", filename);
    Ok(())
}
fn cleanup_and_abort(backup_dir: &PathBuf, backups: &[(PathBuf, PathBuf)]) -> Result<()> {
    // remove temp backups to avoid clutter on cancel
    for (_, backup) in backups {
        let _ = fs::remove_file(backup);
    }
    fs_helpers::cleanup_backup_dir(backup_dir);
    // no-op for now; leaving for symmetry
    Ok(())
}

fn restore_and_cleanup(backups: &[(PathBuf, PathBuf)], backup_dir: &PathBuf) -> Result<()> {
    fs_helpers::restore_from_backups(backups)?;
    fs_helpers::cleanup_backup_dir(backup_dir);
    Ok(())
}
