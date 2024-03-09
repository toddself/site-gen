use std::fs;

use clap::Parser;
use color_eyre::Result;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

mod builder;
mod helpers;
use crate::builder::Builder;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Parser)]
enum Action {
    Build {
        #[command(subcommand)]
        action: BuildCommand,
    },
    // Validate {
    //     #[command(subcommand)]
    //     action: Validate
    // }
}

#[derive(Debug, Clone, clap::Subcommand)]
#[command(version, about, long_about = None)]
enum BuildCommand {
    Build {
        /// Path to config file
        #[arg(short, long)]
        config: Option<String>,

        /// How many entries per page
        #[arg(short, long)]
        entries: Option<u8>,

        /// Directory for templates
        #[arg(short = 'p', long)]
        template_dir: Option<String>,

        /// Source directory for markdown files
        src: Option<String>,

        /// Destination for HTML output
        dest: Option<String>,

        /// Title for the site
        #[arg(short, long)]
        title: Option<String>,

        /// How long should entries be in the RSS feed
        #[arg(long)]
        truncate: Option<u32>,

        /// Description for the site
        #[arg(long)]
        description: Option<String>,

        /// URL for the site
        #[arg(short, long)]
        url: Option<Url>,

        /// Author for site
        #[arg(short, long)]
        author: Option<String>,

        /// Social share image for site
        #[arg(long)]
        share_image: Option<String>,
    },
}

#[derive(Debug, Error)]
enum ParserError {
    #[error("You must provide {0} either in a config or via the CLI arguments")]
    MissingArg(String),
}

#[derive(Debug, Deserialize)]
struct Config {
    entries: u8,
    template_dir: String,
    src: String,
    dest: String,
    title: String,
    url: Url,
    truncate: Option<u32>,
    description: Option<String>,
    author: Option<String>,
    share_image: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn parse_args(
    src: Option<String>,
    dest: Option<String>,
    entries: Option<u8>,
    template_dir: Option<String>,
    title: Option<String>,
    truncate: Option<u32>,
    description: Option<String>,
    url: Option<Url>,
    author: Option<String>,
    share_image: Option<String>,
) -> Result<Config> {
    let entries = entries.ok_or(ParserError::MissingArg("entries".to_string()))?;
    let template_dir = template_dir.ok_or(ParserError::MissingArg("template_dir".to_string()))?;
    let src = src.ok_or(ParserError::MissingArg("src".to_string()))?;
    let dest = dest.ok_or(ParserError::MissingArg("dest".to_string()))?;
    let title = title.ok_or(ParserError::MissingArg("title".to_string()))?;
    let url = url.ok_or(ParserError::MissingArg("url".to_string()))?;
    Ok(Config {
        entries,
        template_dir,
        src,
        dest,
        title,
        truncate,
        description,
        url,
        author,
        share_image,
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.action {
        Action::Build {
            action:
                BuildCommand::Build {
                    src,
                    dest,
                    config,
                    entries,
                    template_dir,
                    title,
                    truncate,
                    description,
                    url,
                    author,
                    share_image,
                },
        } => {
            let config_data: Config = match config {
                Some(config) => {
                    let data = fs::read_to_string(config)?;
                    let mut data: Config = toml::from_str(&data)?;
                    if let Some(entries) = entries {
                        data.entries = entries;
                    }
                    if let Some(template_dir) = template_dir {
                        data.template_dir = template_dir;
                    }
                    if let Some(src) = src {
                        data.src = src;
                    }
                    if let Some(dest) = dest {
                        data.dest = dest;
                    }
                    if let Some(title) = title {
                        data.title = title;
                    }
                    if let Some(url) = url {
                        data.url = url;
                    }

                    data.truncate = truncate;
                    data.description = description;
                    data.author = author;
                    data
                }
                None => parse_args(
                    src,
                    dest,
                    entries,
                    template_dir,
                    title,
                    truncate,
                    description,
                    url,
                    author,
                    share_image,
                )?,
            };

            let mut b = Builder::new(config_data)?;

            match b.build() {
                Ok(_a) => println!("Blog built!"),
                Err(e) => println!("{:?}", e),
            };
            Ok(())
        }
    }
}
