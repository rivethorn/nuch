use anyhow::Result;
use inquire::Select;
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use crate::config::CollectionPaths;

pub fn list_blogs(dir: &Path, exclude_dir: Option<&CollectionPaths>) -> Result<Option<PathBuf>> {
    let mut markdown_files: Vec<_> = Vec::new();
    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(ex) = exclude_dir {
                    let dest = ex.files.join(path.file_name().unwrap());
                    if dest.exists() {
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

    let selected_index = markdown_files
        .iter()
        .position(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == selected_name)
                .unwrap_or(false)
        })
        .expect("Selected file should exist");

    Ok(Some(markdown_files[selected_index].clone()))
}

pub fn list_colletctions(cols: Vec<CollectionPaths>) -> Result<Option<CollectionPaths>> {
    // if there is only one collection, select it automatically
    if cols.len() == 1 {
        return Ok(Some(cols[0].clone()));
    }

    let collection_names: Vec<_> = cols.iter().map(|c| c.name.clone()).collect();

    let selection = Select::new("First, select your collection:", collection_names)
        .with_vim_mode(true)
        .without_filtering()
        .with_help_message("hjkl to move, enter, esc to quit")
        .prompt()?;

    // let selected_name = match selection {
    //     Some(name) => name,
    //     None => {
    //         println!("Cancelled.");
    //         return Ok(None);
    //     }
    // };

    let selected_index = cols
        .iter()
        .position(|c| c.name == selection)
        .expect("Selected collection should exist");

    Ok(Some(cols[selected_index].clone()))
}
