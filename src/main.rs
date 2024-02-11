use std::fs;

use clap::Parser;
use color_eyre::Result;
use serde::Deserialize;

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
    src: String,

    /// Destination for HTML output
    dest: String,

    /// Title for the site
    #[arg(short, long, default_value = "a blog")]
    title: String,

    /// How long should entries be in the RSS feed
    #[arg(short = 'u', long)]
    truncate: Option<u32>,
}

fn main() -> Result<()> {
    let opts = Opt::parse();

    let config_data = if let Some(config) = opts.config {
        let data = fs::read_to_string(config)?;
        toml::from_str(&data)?
    } else {
        opts
    };

    let mut b = Builder::new(config_data)?;

    match b.build() {
        Ok(_a) => println!("Blog built!"),
        Err(e) => println!("{:?}", e),
    };
    Ok(())
}
