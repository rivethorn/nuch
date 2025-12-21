mod config;
mod fs;
mod git;
mod publish;
mod ui;

use anyhow::Result;
use clap::Parser;

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
            if let Some(selected) = ui::list_blogs(&app_paths.ready, Some(&app_paths.published))? {
                publish::publish_selected(&selected, &app_paths)?;
            }
        }
        Some(Command::Delete) => {
            if let Some(selected) = ui::list_blogs(&app_paths.published, None)? {
                publish::delete_selected(&selected, &app_paths)?;
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
