use anyhow::Result;
use inquire::Select;
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use crate::config::CollectionPaths;

pub fn list_blogs(dir: &Path, exclude_dir: Option<&CollectionPaths>) -> Result<Option<PathBuf>> {
    let mut content_files: Vec<_> = Vec::new();
    let supported_exts = ["md", "yaml", "yml", "json", "csv"];
    
    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            let is_supported = path.is_file()
                && path.extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| supported_exts.contains(&ext));
            
            let is_excluded = exclude_dir
                .map(|ex| ex.files.join(path.file_name().unwrap()).exists())
                .unwrap_or(false);
            
            if is_supported && !is_excluded {
                content_files.push(path);
            }
        }
    }

    if content_files.is_empty() {
        println!("No supported files found.");
        return Ok(None);
    }

    let names: Vec<_> = content_files
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect();

    let selection = Select::new("Select a file:", names)
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

    let selected_index = content_files
        .iter()
        .position(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == selected_name)
                .unwrap_or(false)
        })
        .expect("Selected file should exist");

    Ok(Some(content_files[selected_index].clone()))
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
