use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{env::home_dir, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub working_dir: String,
    pub publishing_dir: String,
    pub working_images_dir: Option<String>,
    pub publishing_images_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub ready: PathBuf,
    pub published: PathBuf,
    pub working_images: Option<PathBuf>,
    pub publishing_images: Option<PathBuf>,
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

    if cfg.working_dir.trim().is_empty() {
        return Err(anyhow::anyhow!("'working_dir' in config is empty."));
    }
    if cfg.publishing_dir.trim().is_empty() {
        return Err(anyhow::anyhow!("'publishing_dir' in config is empty."));
    }

    let ready_path = resolve_dir(&cfg.working_dir);
    let published_path = resolve_dir(&cfg.publishing_dir);

    let working_images = cfg.working_images_dir.as_ref().map(|s| resolve_dir(s));
    let publishing_images = cfg.publishing_images_dir.as_ref().map(|s| resolve_dir(s));

    // Validate markdown dirs
    let mut errs: Vec<String> = Vec::new();
    if !super::fs::dir_has_markdown(&ready_path).unwrap_or(false) {
        errs.push(format!(
            "No Markdown files found in working_dir: {}",
            ready_path.display()
        ));
    }
    if !super::fs::dir_has_markdown(&published_path).unwrap_or(false) {
        errs.push(format!(
            "No Markdown files found in publishing_dir: {}",
            published_path.display()
        ));
    }

    if let Some(p) = &working_images {
        if !p.is_dir() {
            errs.push(format!(
                "working_images_dir does not exist or is not a directory: {}",
                p.display()
            ));
        }
    }
    if let Some(p) = &publishing_images {
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
