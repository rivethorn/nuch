use anyhow::Result;
use clap::Parser;
use inquire::{Confirm, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{env::home_dir, path::Path};

#[derive(Parser)]
struct Args {
    /// Generate a sample config file in the config directory if none exists
    #[arg(long = "config")]
    generate_config: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    Publish,
    Delete,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    working_dir: String,
    publishing_dir: String,
    working_images_dir: Option<String>,
    publishing_images_dir: Option<String>,
}

/// Determine the path to the config file.
/// Returns None if the config directory cannot be determined.
/// Uses XDG_CONFIG_HOME or falls back to ~/.config/nuch/config.toml on failure.
fn config_file_path() -> Option<PathBuf> {
    // Prefer XDG_CONFIG_HOME if set, otherwise fall back to ~/.config/nuch/config.toml
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("nuch").join("config.toml"));
    }

    if let Some(home) = home_dir() {
        return Some(home.join(".config").join("nuch").join("config.toml"));
    }

    None
}

/// Resolved application paths returned from loading the config.
struct AppPaths {
    ready: PathBuf,
    published: PathBuf,
    working_images: Option<PathBuf>,
    publishing_images: Option<PathBuf>,
}

/// Load, validate, and (optionally) generate the config file.
/// Returns Ok(None) if `generate` was true and a sample config was written (caller should exit).
/// Returns Ok(Some(AppPaths)) when config is present, parsed and validated.
fn load_config(generate: bool) -> Result<Option<AppPaths>> {
    let config_path = match config_file_path() {
        Some(p) => p,
        None => {
            if generate {
                eprintln!("Could not determine config directory on this platform.");
                return Ok(None);
            } else {
                return Err(anyhow::anyhow!(
                    "Could not determine config directory on this platform. Use --config to create a sample."
                ));
            }
        }
    };

    let config_dir = config_path.parent().unwrap();

    if generate {
        // create config dir if needed
        fs::create_dir_all(config_dir)?;
        if config_path.exists() {
            println!("Config already exists at {}", config_path.display());
        } else {
            let sample = Config {
                working_dir: "Documents/writings".to_string(),
                publishing_dir: "your-site/content".to_string(),
                working_images_dir: Some("Documents/writings/images".to_string()),
                publishing_images_dir: Some("your-site/public/images".to_string()),
            };
            let toml_str = toml::to_string_pretty(&sample)?;
            let mut f = fs::File::create(&config_path)?;
            f.write_all(toml_str.as_bytes())?;
            println!("Wrote sample config to {}", config_path.display());
        }
        return Ok(None);
    }

    // Require config file to exist
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {}. Run with --config to create one.",
            config_path.display()
        ));
    }

    // Read and parse config (fatal on error)
    let s = fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config {}: {}", config_path.display(), e))?;
    let cfg: Config = toml::from_str(&s)
        .map_err(|e| anyhow::anyhow!("Failed to parse config {}: {}", config_path.display(), e))?;

    if cfg.working_dir.trim().is_empty() {
        return Err(anyhow::anyhow!("'working_dir' in config is empty."));
    }
    if cfg.publishing_dir.trim().is_empty() {
        return Err(anyhow::anyhow!("'publishing_dir' in config is empty."));
    }

    // Resolve into absolute paths
    let ready_path = resolve_dir(&cfg.working_dir);
    let published_path = resolve_dir(&cfg.publishing_dir);

    let working_images = match &cfg.working_images_dir {
        Some(s) => Some(resolve_dir(s)),
        None => None,
    };
    let publishing_images = match &cfg.publishing_images_dir {
        Some(s) => Some(resolve_dir(s)),
        None => None,
    };

    // Validate that the configured directories exist and contain Markdown files
    let mut errs: Vec<String> = Vec::new();

    match dir_has_markdown(&ready_path) {
        Ok(true) => {}
        Ok(false) => errs.push(format!(
            "No Markdown files found in working_dir: {}",
            ready_path.display()
        )),
        Err(e) => errs.push(format!(
            "Failed to read working_dir {}: {}",
            ready_path.display(),
            e
        )),
    }

    match dir_has_markdown(&published_path) {
        Ok(true) => {}
        Ok(false) => errs.push(format!(
            "No Markdown files found in publishing_dir: {}",
            published_path.display()
        )),
        Err(e) => errs.push(format!(
            "Failed to read publishing_dir {}: {}",
            published_path.display(),
            e
        )),
    }

    // If image dirs are set, ensure they at least exist as directories
    if let Some(ref p) = working_images {
        if !p.is_dir() {
            errs.push(format!(
                "working_images_dir does not exist or is not a directory: {}",
                p.display()
            ));
        }
    }
    if let Some(ref p) = publishing_images {
        if !p.is_dir() {
            errs.push(format!(
                "publishing_images_dir does not exist or is not a directory: {}",
                p.display()
            ));
        }
    }

    if !errs.is_empty() {
        return Err(anyhow::anyhow!(errs.join("; ")));
    }

    Ok(Some(AppPaths {
        ready: ready_path,
        published: published_path,
        working_images,
        publishing_images,
    }))
}

/// Resolve a directory path, expanding '~' to home if needed.
/// Returns an absolute PathBuf.
/// If the input is already absolute, it is returned as-is.
fn resolve_dir(dir: &str) -> PathBuf {
    let p = Path::new(dir);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        let home = home_dir().unwrap();
        Path::new(&home).join(dir)
    }
}

/// Check if the specified directory contains any Markdown (.md) files.
/// Returns Ok(true) if at least one Markdown file is found, Ok(false) if none are found,
/// or an Err if there was an I/O error accessing the directory.
fn dir_has_markdown(dir: &std::path::Path) -> Result<bool, std::io::Error> {
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

/// List Markdown files in the specified directory and prompt user to select one.
/// If `exclude_dir` is provided, files that already exist (by filename) in that directory
/// will be omitted from the list (used when publishing so we don't show already published files).
/// Returns Ok(Some(PathBuf)) with the selected file, Ok(None) if the user cancelled, or an Err on failure.
fn list_blogs(
    dir: &std::path::Path,
    exclude_dir: Option<&std::path::Path>,
) -> Result<Option<PathBuf>> {
    let mut markdown_files: Vec<_> = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                // If exclude_dir provided, skip files whose name exists in exclude_dir
                if let Some(ex) = exclude_dir {
                    let dest = ex.join(path.file_name().unwrap());
                    if dest.exists() {
                        // skip already-published file
                        continue;
                    }
                }
                markdown_files.push(path);
            }
        }
    }

    if markdown_files.is_empty() {
        println!("No Markdown files found.");
        return Ok(None);
    }

    let names: Vec<_> = markdown_files
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    let selection = Select::new("Select a Markdown file:", names)
        .with_vim_mode(true)
        .without_filtering()
        .with_help_message("hjkl to move, enter, esc to quit")
        .prompt_skippable()?;
    let selected_name = match selection {
        Some(name) => name,
        None => {
            println!("Cancelled.");
            return Ok(None);
        }
    };

    // Find the index of the selected name (safe, handles duplicates gracefully)
    let selected_index = markdown_files
        .iter()
        .position(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == selected_name)
                .unwrap_or(false)
        })
        .expect("Selected file should exist");

    let selected_path = markdown_files[selected_index].clone();
    Ok(Some(selected_path))
}

fn main() -> Result<()> {
    let args = Args::parse();

    let paths = load_config(args.generate_config)?;
    if paths.is_none() {
        // generation mode: sample config written (or couldn't determine path and we already printed an error)
        return Ok(());
    }

    let app_paths = paths.unwrap();

    match args.command {
        Some(Command::Publish) => {
            if let Some(selected) = list_blogs(&app_paths.ready, Some(&app_paths.published))? {
                publish_selected(&selected, &app_paths)?;
            }
        }
        Some(Command::Delete) => {
            if let Some(selected) = list_blogs(&app_paths.published, None)? {
                delete_selected(&selected, &app_paths)?;
            }
        }
        None => {
            return Err(anyhow::anyhow!(
                "No command provided. Use 'publish' or 'delete'."
            ));
        }
    }

    Ok(())
}

fn perform_action(path: &std::path::Path) -> Result<()> {
    println!("Selected: {:?}", path.display());
    // Example action: read and print content
    let content = std::fs::read_to_string(path)?;
    println!(
        "Content preview:\n{}",
        content.lines().take(5).collect::<Vec<_>>().join("\n")
    );
    Ok(())
}

/// Copy the selected markdown and associated images (if configured) and run git steps.
fn publish_selected(selected: &std::path::Path, paths: &AppPaths) -> Result<()> {
    let filename = selected
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Destination markdown path
    let dest_md = paths.published.join(filename);
    if dest_md.exists() {
        return Err(anyhow::anyhow!(
            "Destination markdown already exists: {}",
            dest_md.display()
        ));
    }

    // Copy markdown
    fs::create_dir_all(&paths.published)?;
    fs::copy(selected, &dest_md)
        .map_err(|e| anyhow::anyhow!("Failed to copy markdown to {}: {}", dest_md.display(), e))?;

    // Keep track of created files for rollback
    let mut created: Vec<PathBuf> = vec![dest_md.clone()];

    // Copy images if configured
    if let (Some(src_images), Some(dst_images)) = (&paths.working_images, &paths.publishing_images)
    {
        // ensure destination directory exists
        fs::create_dir_all(dst_images)?;

        let stem = selected
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename stem"))?;

        let stem_lower = stem.to_lowercase();
        let img_exts = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

        for entry in fs::read_dir(src_images)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            let name_lower = name.to_lowercase();

            // check starts with stem and ends with valid extension
            if !name_lower.starts_with(&stem_lower) {
                continue;
            }
            if !img_exts.iter().any(|ext| name_lower.ends_with(ext)) {
                continue;
            }

            let dest_img = dst_images.join(entry.file_name());
            if dest_img.exists() {
                // Don't overwrite — abort and rollback
                // Attempt rollback
                for f in &created {
                    let _ = fs::remove_file(f);
                }
                return Err(anyhow::anyhow!(
                    "Image already exists at destination {} — aborting",
                    dest_img.display()
                ));
            }

            fs::copy(&p, &dest_img).map_err(|e| {
                // rollback files we already created
                for f in &created {
                    let _ = fs::remove_file(f);
                }
                anyhow::anyhow!("Failed to copy image {}: {}", p.display(), e)
            })?;
            created.push(dest_img);
        }
    }

    // Determine site root and run git steps
    let site_root = get_site_root(&paths.published);
    if let Err(e) = run_git_steps(&site_root, &format!("Add {} to blog", filename)) {
        // rollback copies
        for f in &created {
            let _ = fs::remove_file(f);
        }
        return Err(e);
    }

    println!("Published {} successfully", filename);
    Ok(())
}

/// Delete a selected published blog and its images. Optionally backup to working dirs first.
fn delete_selected(selected: &std::path::Path, paths: &AppPaths) -> Result<()> {
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
    let working_md = paths.ready.join(filename);
    let mut backup_files: Vec<PathBuf> = Vec::new();
    let mut backup_made = false;

    if !working_md.exists() {
        // Prompt for backup
        let ask = format!(
            "'{}' not found in working dir. Create backup in working dir?",
            filename
        );
        if Confirm::new(&ask).with_default(true).prompt()? {
            // Copy markdown
            fs::create_dir_all(&paths.ready)?;
            let copied = copy_file_to(&selected.to_path_buf(), &paths.ready)?;
            backup_files.push(copied.clone());
            backup_made = true;

            // Copy images if possible
            if let (Some(pub_imgs), Some(work_imgs)) =
                (&paths.publishing_images, &paths.working_images)
            {
                let images = matching_images_for_stem(&stem_lower, pub_imgs)?;
                if !images.is_empty() {
                    fs::create_dir_all(work_imgs)?;
                    for img in images.iter() {
                        let dest = work_imgs.join(img.file_name().unwrap());
                        if dest.exists() {
                            // abort and attempt to clean up copied files
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

            println!("Backup created in {}", paths.ready.display());
        } else {
            println!("Proceeding without backup.");
        }
    } else {
        println!("File exists in working dir; skipping backup.");
    }

    // Gather list of images to delete in publishing_images
    let mut to_delete: Vec<PathBuf> = Vec::new();
    to_delete.push(selected.to_path_buf());
    if let Some(pub_imgs) = &paths.publishing_images {
        let images = matching_images_for_stem(&stem_lower, pub_imgs)?;
        for img in images {
            to_delete.push(img);
        }
    }

    // Delete files (stop on first error)
    for p in &to_delete {
        if p.exists() {
            fs::remove_file(p)
                .map_err(|e| anyhow::anyhow!("Failed to remove {}: {}", p.display(), e))?;
        }
    }

    // Run git steps
    let site_root = get_site_root(&paths.published);
    if let Err(e) = run_git_steps(&site_root, &format!("Remove {} from blog", filename)) {
        // try to restore deleted files if we made a backup
        if backup_made {
            for f in &backup_files {
                let name = f.file_name().unwrap();
                let restore_dest = paths.published.join(name);
                if restore_dest.exists() {
                    continue;
                }
                if let Err(rerr) = fs::copy(f, &restore_dest) {
                    eprintln!(
                        "Failed to restore {} from backup: {}",
                        restore_dest.display(),
                        rerr
                    );
                }
            }
        }
        return Err(e);
    }

    println!("Deleted {} and corresponding images", filename);
    Ok(())
}

fn get_site_root(published: &PathBuf) -> PathBuf {
    for anc in published.ancestors() {
        if let Some(name) = anc.file_name().and_then(|s| s.to_str()) {
            if name == "content" {
                return anc.parent().unwrap().to_path_buf();
            }
        }
    }
    published.parent().unwrap().to_path_buf()
}

fn run_git_steps(site_root: &PathBuf, commit_msg: &str) -> Result<()> {
    // Ensure it's a git repo
    let git_check = std::process::Command::new("git")
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

    let git_add = std::process::Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(site_root)
        .output()?;
    if !git_add.status.success() {
        return Err(anyhow::anyhow!(
            "git add failed: {}",
            String::from_utf8_lossy(&git_add.stderr)
        ));
    }

    let git_commit = std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(commit_msg)
        .current_dir(site_root)
        .output()?;
    if !git_commit.status.success() {
        return Err(anyhow::anyhow!(
            "git commit failed: {}",
            String::from_utf8_lossy(&git_commit.stderr)
        ));
    }

    let git_push = std::process::Command::new("git")
        .arg("push")
        .current_dir(site_root)
        .output()?;
    if !git_push.status.success() {
        return Err(anyhow::anyhow!(
            "git push failed: {}",
            String::from_utf8_lossy(&git_push.stderr)
        ));
    }

    Ok(())
}

/// Return list of image files in `dir` whose name starts with `stem_lower` (case-insensitive) and end with known image extensions.
fn matching_images_for_stem(
    stem_lower: &str,
    dir: &std::path::Path,
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

fn copy_file_to(src: &PathBuf, dst_dir: &PathBuf) -> Result<PathBuf> {
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

fn remove_files(paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        if p.exists() {
            fs::remove_file(p)
                .map_err(|e| anyhow::anyhow!("Failed to remove {}: {}", p.display(), e))?;
        }
    }
    Ok(())
}
