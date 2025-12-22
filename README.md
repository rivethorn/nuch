# NUCH (NUxt Content Handler)

A small CLI to help manage Markdown content and associated images for Nuxt Content sites.

<img src="res/icon.png" alt="NUCH Icon" width="128" height="128" style="display: block; margin: 1.5rem auto;"/>

![Static Badge](https://img.shields.io/badge/NUCH-black?style=for-the-badge&logo=markdown)
[![Crates.io Version](https://img.shields.io/crates/v/nuch?style=for-the-badge&logo=rust&labelColor=black&color=black)](https://crates.io/crates/nuch)

![GitHub deployments](https://img.shields.io/github/deployments/rivethorn/nuch/production?style=for-the-badge&logo=github&label=GitHub%20Action&labelColor=black)

## Quick start

You can install via `cargo`:

```bash
cargo install nuch
```

or you can install prebuilt binaries via shell script

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/rivethorn/nuch/releases/download/v1.0.1/nuch-installer.sh | sh
```

You can also donwload the binary from [Releases](https://github.com/rivethorn/nuch/releases)

### Build from source

- Requirements: Rust toolchain (cargo), and system `git` on PATH.

- Build and run:

```bash
# Build
cargo build --release

# Run (shows help)
cargo run -- --help

# Create a sample config (writes to XDG_CONFIG_HOME/nuch/config.toml or ~/.config/nuch/config.toml)
cargo run -- --config
```

## Typical usage:

```bash
# Publish (interactive): selects a markdown file from your configured working dir
nuch publish

# Delete (interactive): select a published post to remove
nuch delete
```

> [!WARNING]
> The tool **requires a valid config file** at `XDG_CONFIG_HOME/nuch/config.toml` or `~/.config/nuch/config.toml`.
> 
> Use `--config` to generate a sample.

## Config file (TOML)

The config describes your working and publishing directories and optional image directories. Example sample written by `--config`:

```toml
working_dir = "Documents/writings"
publishing_dir = "your-site/content"
working_images_dir = "Documents/writings/images"
publishing_images_dir = "your-site/public/images"
```

- **working_dir** (required): directory containing your drafts/ready-for-publish Markdown files.
- **publishing_dir** (required): your predefined collection, usually inside `content` directory (where published markdown should be copied).
- **working_images_dir** (optional): directory holding images referenced by your working markdown.
- **publishing_images_dir** (optional): directory under the site where images are stored.

The tool validates that `working_dir` and `publishing_dir` exist and contain at least one `.md` file.

## Development notes

- Main modules:

  - `src/config.rs` — config parsing and validation
  - `src/fs.rs` — filesystem helpers (copy, backup, image matching)
  - `src/publish.rs` — publish/delete flows (interactive); includes test-only non-interactive helpers
  - `src/git.rs` — git wrapper helpers
  - `src/ui.rs` — user prompts & listing

- Code style: Rust 2024 edition, uses `clap` for CLI and `inquire` for interactive prompts.

## Contributing

Open a PR or issue for bug fixes or feature ideas. Add tests for any changes that touch behavior.
