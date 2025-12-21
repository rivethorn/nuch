use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;
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
    command: Command,
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

fn config_file_path() -> Option<PathBuf> {
    if let Some(proj) = ProjectDirs::from("", "", "nuch") {
        Some(proj.config_dir().join("config.toml"))
    } else {
        None
    }
}

fn resolve_dir(dir: &str) -> PathBuf {
    let p = Path::new(dir);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        let home = home_dir().unwrap();
        Path::new(&home).join(dir)
    }
}

/// List Markdown files in the specified directory and prompt user to select one.
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

    // Defaults (relative to home directory)
    let mut ready = "Documents/blog".to_string();
    let mut published = "Documents/GitHub/hq/content/writings".to_string();

    if let Some(config_path) = config_file_path() {
        let config_dir = config_path.parent().unwrap();
        if args.generate_config {
            // create config dir if needed
            fs::create_dir_all(config_dir)?;
            if config_path.exists() {
                println!("Config already exists at {}", config_path.display());
            } else {
                let sample = Config {
                    working_dir: ready.clone(),
                    publishing_dir: published.clone(),
                };
                let toml_str = toml::to_string_pretty(&sample)?;
                let mut f = fs::File::create(&config_path)?;
                f.write_all(toml_str.as_bytes())?;
                println!("Wrote sample config to {}", config_path.display());
            }
            return Ok(());
        }

        // If config exists try to parse it and override defaults
        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(s) => match toml::from_str::<Config>(&s) {
                    Ok(cfg) => {
                        if !cfg.working_dir.trim().is_empty() {
                            ready = cfg.working_dir;
                        }
                        if !cfg.publishing_dir.trim().is_empty() {
                            published = cfg.publishing_dir;
                        }
                    }
                    Err(e) => eprintln!("Failed to parse config: {}. Using defaults.", e),
                },
                Err(e) => eprintln!("Failed to read config: {}. Using defaults.", e),
            }
        }
    } else if args.generate_config {
        eprintln!("Could not determine config directory on this platform.");
        return Ok(());
    }

    // Resolve into absolute paths
    let ready_path = resolve_dir(&ready);
    let published_path = resolve_dir(&published);

    match args.command {
        Command::Publish => list_blogs(&ready_path)?,
        Command::Delete => list_blogs(&published_path)?,
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
