use anyhow::Result;
use clap::Parser;
use inquire::Select;
use std::{env::home_dir, path::Path};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Publish,
    Delete,
}

/// List Markdown files in the specified directory and prompt user to select one.
fn list_blogs(dir: &str) -> Result<()> {
    let home_dir = home_dir().unwrap();
    let dir = Path::new(&home_dir).join(dir);

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
        .with_help_message("hjkl to move, enter to select, esc to quit")
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
        .position(|p| p.to_string_lossy() == selected_name.as_str())
        .expect("Selected file should exist");

    let selected_path = &markdown_files[selected_index];
    perform_action(selected_path)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    const READY_BLOGS_DIR: &str = "Documents/blog";
    const PUBLISHED_BLOGS_DIR: &str = "Documents/GitHub/hq/content/writings";

    match args.command {
        Command::Publish => list_blogs(READY_BLOGS_DIR)?,
        Command::Delete => list_blogs(PUBLISHED_BLOGS_DIR)?,
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
