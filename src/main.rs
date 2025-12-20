use std::{env::home_dir, error::Error, fs::read_dir, path::PathBuf};

use crate::tui::run_tui;
mod tui;
fn handle_paths(home_dir: PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
    const BLOG_DIR: &str = "Documents/blog";
    const CONTENT_DIR: &str = "Documents/GitHub/hq/content/writings";
    let blog_dir = home_dir.join(BLOG_DIR);
    let content_dir = home_dir.join(CONTENT_DIR);

    let blogs = read_dir(blog_dir)?
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "md") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let content = read_dir(content_dir)?
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "md") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok((blogs, content))
}

fn main() {
    let home_dir = home_dir().unwrap();
    let (blogs, content) = handle_paths(home_dir).unwrap();
    run_tui(blogs, content);
}
