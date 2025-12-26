use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{env::home_dir, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkingConfig {
    pub files: String,
    pub images: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CollectionConfig {
    pub name: String,
    pub files: String,
    pub images: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub working: WorkingConfig,
    #[serde(default)]
    pub collection: Vec<CollectionConfig>,
}

#[derive(Debug, Clone)]
pub struct CollectionPaths {
    pub name: String,
    pub files: PathBuf,
    pub images: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    // working area (local drafts)
    pub working_files: PathBuf,
    pub working_images: Option<PathBuf>,
    // collections (publishing targets)
    pub collections: Vec<CollectionPaths>,
}

pub fn config_file_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("nuch").join("config.toml"));
    }

    if let Some(home) = home_dir() {
        return Some(home.join(".config").join("nuch").join("config.toml"));
    }

    None
}

pub fn resolve_dir(dir: &str) -> PathBuf {
    let p = Path::new(dir);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        let home = home_dir().unwrap();
        Path::new(&home).join(dir)
    }
}

/// Load, validate, and (optionally) generate the config file.
/// Returns Ok(None) if `generate` was true and a sample config was written (caller should exit).
/// Returns Ok(Some(AppPaths)) when config is present, parsed and validated.
pub fn load_config(generate: bool) -> Result<Option<AppPaths>> {
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
        fs::create_dir_all(config_dir)?;
        if config_path.exists() {
            println!("Config already exists at {}", config_path.display());
        } else {
            // sample config
            let sample = Config {
                working: WorkingConfig {
                    files: "Documents/writings".to_string(),
                    images: Some("Documents/writings/images".to_string()),
                },
                collection: vec![
                    CollectionConfig {
                        name: "writing".to_string(),
                        files: "your-site/content".to_string(),
                        images: Some("your-site/public/images".to_string()),
                    },
                    CollectionConfig {
                        name: "blogs".to_string(),
                        files: "your-site/content/blogs".to_string(),
                        images: None,
                    },
                ],
            };
            let toml_str = toml::to_string_pretty(&sample)?;
            let mut f = fs::File::create(&config_path)?;
            f.write_all(toml_str.as_bytes())?;
            println!("Wrote sample config to {}", config_path.display());
        }
        return Ok(None);
    }

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {}. Run with --config to create one.",
            config_path.display()
        ));
    }

    let s = fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config {}: {}", config_path.display(), e))?;
    let cfg: Config = toml::from_str(&s)
        .map_err(|e| anyhow::anyhow!("Failed to parse config {}: {}", config_path.display(), e))?;

    // Validate working section
    if cfg.working.files.trim().is_empty() {
        return Err(anyhow::anyhow!("'working.files' in config is empty."));
    }

    // Resolve working paths
    let working_files_path = resolve_dir(&cfg.working.files);
    let working_images_path = cfg.working.images.as_ref().map(|s| resolve_dir(s));

    // Validate working dir exists and contains markdown
    let mut errs: Vec<String> = Vec::new();
    if !working_files_path.is_dir() {
        errs.push(format!(
            "working.files does not exist or is not a directory: {}",
            working_files_path.display()
        ));
    } else if !super::fs::dir_has_supported_files(&working_files_path).unwrap_or(false) {
        errs.push(format!(
            "No supported files (.md, .yaml, .yml, .json, .csv) found in working.files: {}",
            working_files_path.display()
        ));
    }

    if let Some(p) = &working_images_path
        && !p.is_dir()
    {
        errs.push(format!(
            "working.images does not exist or is not a directory: {}",
            p.display()
        ));
    }

    // Validate collections
    let mut seen_names = std::collections::HashSet::new();
    let mut collection_paths: Vec<CollectionPaths> = Vec::new();

    for col in &cfg.collection {
        if col.name.trim().is_empty() {
            errs.push("A collection has an empty 'name' field".to_string());
            continue;
        }
        if !seen_names.insert(col.name.clone()) {
            errs.push(format!("Duplicate collection name: {}", col.name));
            continue;
        }

        if col.files.trim().is_empty() {
            errs.push(format!("Collection '{}' has empty 'files' path", col.name));
            continue;
        }

        let files_path = resolve_dir(&col.files);
        let images_path = col.images.as_ref().map(|s| resolve_dir(s));

        if !files_path.is_dir() {
            errs.push(format!(
                "Collection '{}' files path does not exist or is not a directory: {}",
                col.name,
                files_path.display()
            ));
        }

        if let Some(p) = &images_path
            && !p.is_dir()
        {
            errs.push(format!(
                "Collection '{}' images path does not exist or is not a directory: {}",
                col.name,
                p.display()
            ));
        }

        collection_paths.push(CollectionPaths {
            name: col.name.clone(),
            files: files_path,
            images: images_path,
        });
    }

    if !errs.is_empty() {
        return Err(anyhow::anyhow!(errs.join("; ")));
    }

    Ok(Some(AppPaths {
        working_files: working_files_path,
        working_images: working_images_path,
        collections: collection_paths,
    }))
}
