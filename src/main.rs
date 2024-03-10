use std::{fs, path::PathBuf};

use chrono::Local;
use clap::Parser;
use color_eyre::Result;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

mod builder;
mod helpers;
mod logger;

use builder::{Builder, PageMetadata, HEADER_DELIMITER};
use logger::log_format_pretty;

const CONFIG_DEFAULT: &str = ".site-gen.toml";

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    arg: Action,
}

#[derive(Debug, Parser)]
#[allow(variant_size_differences, clippy::large_enum_variant)]
enum Action {
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
    Create {
        #[arg(short, long)]
        config: Option<String>,

        title: Option<String>,
    },
}

#[derive(Debug, Error)]
enum CliError {
    #[error("You must provide {0} either in a config or via the CLI arguments")]
    MissingArg(String),

    #[error("No config file was found ({0})")]
    MissingConfig(String),
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
    let entries = entries.ok_or(CliError::MissingArg("entries".to_string()))?;
    let template_dir = template_dir.ok_or(CliError::MissingArg("template_dir".to_string()))?;
    let src = src.ok_or(CliError::MissingArg("src".to_string()))?;
    let dest = dest.ok_or(CliError::MissingArg("dest".to_string()))?;
    let title = title.ok_or(CliError::MissingArg("title".to_string()))?;
    let url = url.ok_or(CliError::MissingArg("url".to_string()))?;
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

// TODO: Differentiate between an error parsing and a missing file
fn find_config(p: PathBuf, config: &Option<String>) -> Option<String> {
    let config_path = match config {
        Some(config) => PathBuf::from(config),
        None => p.join(PathBuf::from(CONFIG_DEFAULT)),
    };
    match fs::read_to_string(config_path) {
        Ok(data) => Some(data),
        Err(_) => None,
    }
}

fn main() -> Result<()> {
    env_logger::builder().format(log_format_pretty).try_init()?;

    let args = Args::parse();
    log::debug!("Running: {:?}", args.arg);
    match args.arg {
        Action::Build {
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
        } => {
            let cf = find_config(std::env::current_dir()?, &config);
            let config_data: Config = match cf {
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
                Ok(_) => log::info!("Blog built!"),
                Err(e) => log::error!("{:?}", e),
            };
            Ok(())
        }
        Action::Create { config, title } => {
            let config = find_config(std::env::current_dir()?, &config).ok_or(
                CliError::MissingConfig(config.unwrap_or(CONFIG_DEFAULT.to_string())),
            )?;
            let data: Config = toml::from_str(&config)?;

            let pm = PageMetadata {
                date: Local::now().into(),
                tag_list: None,
                title: "Page Title".to_string(),
                share_image: None,
                hero_image: None,
                author: data.author,
                description: None,
            };
            let meta = toml::to_string(&pm)?;
            let page_data = format!("{HEADER_DELIMITER}\n{meta}{HEADER_DELIMITER}\n");
            let entry_name = format!("{}.md", title.unwrap_or("untitled".to_string()));
            let new_file = PathBuf::from(data.src).join(entry_name);
            fs::write(new_file, page_data)?;
            Ok(())
        }
    }
}
