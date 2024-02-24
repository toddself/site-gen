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
#[command(version, about, long_about = None)]
struct CliArgs {
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
}

#[derive(Debug, Error)]
enum ParserError {
    #[error("You must provide {0} either in a config or via the CLI arguments")]
    MissingArg(String),
}

impl TryInto<Config> for CliArgs {
    type Error = ParserError;
    fn try_into(self) -> std::result::Result<Config, ParserError> {
        let entries = self
            .entries
            .ok_or(ParserError::MissingArg("entries".to_string()))?;
        let template_dir = self
            .template_dir
            .ok_or(ParserError::MissingArg("template_dir".to_string()))?;
        let src = self.src.ok_or(ParserError::MissingArg("src".to_string()))?;
        let dest = self
            .dest
            .ok_or(ParserError::MissingArg("dest".to_string()))?;
        let title = self
            .title
            .ok_or(ParserError::MissingArg("title".to_string()))?;
        let truncate = self.truncate;
        let description = self.description;
        let url = self.url.ok_or(ParserError::MissingArg("url".to_string()))?;
        let author = self.author;
        let share_image = self.share_image;
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

fn main() -> Result<()> {
    let opts = CliArgs::parse();

    let config_data: Config = match opts.config {
        Some(config) => {
            let data = fs::read_to_string(config)?;
            let mut data: Config = toml::from_str(&data)?;
            if let Some(entries) = opts.entries {
                data.entries = entries;
            }
            if let Some(template_dir) = opts.template_dir {
                data.template_dir = template_dir;
            }
            if let Some(src) = opts.src {
                data.src = src;
            }
            if let Some(dest) = opts.dest {
                data.dest = dest;
            }
            if let Some(title) = opts.title {
                data.title = title;
            }
            if let Some(url) = opts.url {
                data.url = url;
            }

            data.truncate = opts.truncate;
            data.description = opts.description;
            data.author = opts.author;
            data
        }
        None => opts.try_into()?,
    };

    let mut b = Builder::new(config_data)?;

    match b.build() {
        Ok(_a) => println!("Blog built!"),
        Err(e) => println!("{:?}", e),
    };
    Ok(())
}
