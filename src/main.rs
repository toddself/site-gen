use std::fs;

use clap::Parser;
use color_eyre::Result;
use serde::Deserialize;
use thiserror::Error;

mod builder;
mod helpers;
use crate::builder::Builder;

#[derive(Debug, Parser, Deserialize)]
#[command(version, about, long_about = None)]
struct Opt {
    /// Path to config file
    #[arg(short, long)]
    config: Option<String>,

    /// How many entries per page
    #[arg(short, long, default_value = "20")]
    entries: u8,

    /// Directory for templates
    #[arg(short = 'p', long, default_value = "templates")]
    template_dir: String,

    /// Source directory for markdown files
    src: Option<String>,

    /// Destination for HTML output
    dest: Option<String>,

    /// Title for the site
    #[arg(short, long, default_value = "a blog")]
    title: String,

    /// How long should entries be in the RSS feed
    #[arg(long)]
    truncate: Option<u32>,

    /// Description for the site
    #[arg(long)]
    description: Option<String>,

    /// URL for the site
    #[arg(short, long)]
    url: Option<String>,

    /// Author for site
    #[arg(short, long)]
    author: Option<String>,
}

#[derive(Debug, Error)]
enum ProgramError {
    #[error("You must provide src, dest and url in either the config or the command-line options")]
    MissingOption,
}

fn main() -> Result<()> {
    let opts = Opt::parse();

    let config_data = if let Some(config) = opts.config {
        let data = fs::read_to_string(config)?;
        toml::from_str(&data)?
    } else {
        opts
    };

    if config_data.src.is_none() || config_data.dest.is_none() || config_data.url.is_none() {
        return Err(ProgramError::MissingOption.into());
    }

    let mut b = Builder::new(config_data)?;

    match b.build() {
        Ok(_a) => println!("Blog built!"),
        Err(e) => println!("{:?}", e),
    };
    Ok(())
}
