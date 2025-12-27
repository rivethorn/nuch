mod config;
mod fs;
mod git;
mod publish;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Generate a sample config file in the config directory if none exists
    #[arg(long = "config")]
    generate_config: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Publish a selected Markdown file from working to publishing directory
    Publish,
    /// Delete a selected Markdown file from publishing directory
    Delete,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let paths = config::load_config(args.generate_config)?;
    if paths.is_none() {
        return Ok(());
    }

    let app_paths = paths.unwrap();

    match args.command {
        Some(Command::Publish) => {
            if let Some(collection) = ui::list_collections(app_paths.collections)?
                && let Some(selected) = ui::list_blogs(&app_paths.working_files, Some(&collection))?
            {
                publish::publish_selected(selected, collection, app_paths.working_images)?;
            }
        }
        Some(Command::Delete) => {
            if let Some(collection) = ui::list_collections(app_paths.collections)?
                && let Some(selected) = ui::list_blogs(&collection.files, None)?
            {
                publish::delete_selected(
                    selected,
                    collection,
                    app_paths.working_files,
                    app_paths.working_images,
                )?;
            }
        }
        None => {
            println!("Error: No command provided. Use 'publish' or 'delete'.\n");

            Args::command().print_help()?;
        }
    }

    Ok(())
}
