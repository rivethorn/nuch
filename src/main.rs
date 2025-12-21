use anyhow::Result;
use clap::Parser;
use inquire::Select;
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

/// Load, validate, and (optionally) generate the config file.
///
/// Returns Ok(None) if `generate` was true and a sample config was written (caller should exit).
/// Returns Ok(Some((ready_path, published_path))) when config is present, parsed and validated.
fn load_config(generate: bool) -> Result<Option<(PathBuf, PathBuf)>> {
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

    if !errs.is_empty() {
        return Err(anyhow::anyhow!(errs.join("; ")));
    }

    Ok(Some((ready_path, published_path)))
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
/// Performs an action on the selected file.
/// Returns Ok(()) on success, or an Err on failure.
fn list_blogs(dir: &std::path::Path) -> Result<()> {
    let dir = dir;

    let mut markdown_files: Vec<_> = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                markdown_files.push(path);
            }
        }
    }

    if markdown_files.is_empty() {
        println!("No Markdown files found.");
        return Ok(());
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
            return Ok(());
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

    let selected_path = &markdown_files[selected_index];
    perform_action(selected_path)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let paths = load_config(args.generate_config)?;
    if paths.is_none() {
        // generation mode: sample config written (or couldn't determine path and we already printed an error)
        return Ok(());
    }

    let (ready_path, published_path) = paths.unwrap();

    match args.command {
        Some(Command::Publish) => list_blogs(&ready_path)?,
        Some(Command::Delete) => list_blogs(&published_path)?,
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
