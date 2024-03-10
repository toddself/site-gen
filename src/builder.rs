use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use chrono::{DateTime, FixedOffset, Local};
use color_eyre::Result;
use comrak::{markdown_to_html, ComrakOptions};
use handlebars::{handlebars_helper, Handlebars};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use truncate_string_at_whitespace::truncate_text;
use url::Url;
use voca_rs::strip::strip_tags;

use crate::helpers::{get_entries, parse_date};
use crate::Config;

#[derive(Debug, Serialize, Clone)]
struct PageData {
    rendered_at: DateTime<FixedOffset>,
    created_at: DateTime<FixedOffset>,
    raw_text: String,
    contents: String,
    tags: Option<Vec<String>>,
    title: String,
    url: String,
    hero_image: Option<String>,
    share_image: Option<String>,
    description: Option<String>,
    site_url: Url,
    domain: String,
    author: Option<String>,
    year: String,
    site_description: Option<String>,
    truncated_contents: String,
}

#[derive(Debug, Serialize, Clone)]
struct PaginationData {
    name: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct IndexData {
    title: String,
    entries: Vec<PageData>,
    published_at: DateTime<FixedOffset>,
    site_url: Url,
    site_description: String,
    domain: String,
    pagination: Vec<PaginationData>,
    share_image: Option<String>,
}

#[derive(Debug, Serialize)]
struct TagData {
    url: String,
    title: String,
    tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageMetadata {
    pub date: DateTime<FixedOffset>,
    pub tag_list: Option<Vec<String>>,
    pub title: String,
    pub share_image: Option<String>,
    pub hero_image: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct Builder<'blog> {
    opts: Config,
    files: Vec<PathBuf>,
    entries: Vec<PageData>,
    hbs: Handlebars<'blog>,
    domain: String,
}

#[derive(Debug, Error)]
enum BuilderError {
    #[error("{0:?} contains invalid unicode identifiers")]
    BadFilename(Box<PathBuf>),

    #[error("URL had no host")]
    BadURL,
}

pub const HEADER_DELIMITER: &str = "---";
const DEFAULT_TRUNCATED: u32 = 300;

impl<'blog> Builder<'blog> {
    pub fn new(opts: Config) -> Result<Builder<'blog>> {
        fs::DirBuilder::new().recursive(true).create(&opts.dest)?;

        let src = PathBuf::from(&opts.src);
        let files = get_entries(&src).unwrap_or_default();

        let mut hbs = Handlebars::new();
        let tmpl_src = PathBuf::from(&opts.template_dir);
        let templates = get_entries(&tmpl_src).unwrap_or_default();

        for tpl_path in templates.iter() {
            if let Some(filename) = tpl_path.to_str() {
                let name = match tpl_path.iter().last() {
                    Some(u) => match u.to_str() {
                        Some(u) => u.split('.').next().unwrap(),
                        None => filename,
                    },
                    None => filename,
                };
                hbs.register_template_file(name, tpl_path)?;
            }
        }
        handlebars_helper!(date: |v: String, f: String| parse_date(&v).format(&f).to_string());
        hbs.register_helper("date", Box::new(date));
        let domain = opts.url.clone();
        let domain = domain.host().ok_or(BuilderError::BadURL)?;

        Ok(Builder {
            opts,
            files,
            entries: vec![],
            hbs,
            domain: domain.to_string(),
        })
    }

    pub fn build(&mut self) -> Result<()> {
        for file in self.files.iter() {
            let entry = self.parse_entry(file)?;
            self.entries.push(entry);
        }

        self.entries.sort_by(|a, b| {
            let bd = b.created_at.signed_duration_since(a.created_at);
            let ad = a.created_at.signed_duration_since(b.created_at);
            bd.cmp(&ad)
        });

        self.build_blog()
    }

    fn build_blog(&self) -> Result<()> {
        let mut count = 0;
        let num_per_page = self.opts.entries;

        // create a list of all the indexes we're gonna output
        let mut pagination: Vec<_> = vec![];
        let num_entries = self.entries.len() as u8;
        if num_entries > num_per_page {
            let mut num_pages = num_entries / num_per_page;
            if num_entries % num_per_page > 0 {
                num_pages += 1;
            }
            for index in 0..num_pages {
                pagination.push(match index {
                    0 => PaginationData {
                        name: "home".to_string(),
                        url: "index.html".to_string(),
                    },
                    _ => PaginationData {
                        name: format!("page {}", index),
                        url: format!("index{}.html", index),
                    },
                });
            }
        }

        // generate the pages
        let now = Local::now();
        let mut rss_data: Vec<_> = vec![];
        let mut tag_map: BTreeMap<String, Vec<TagData>> = BTreeMap::new();
        let dest = PathBuf::from(&self.opts.dest);

        for entry_set in self.entries.chunks(num_per_page.into()) {
            // output individual page, and add to rss and tag dictionaries
            for entry in entry_set {
                let post_data = json!(entry);
                let rendered = self.hbs.render("entry", &post_data)?;
                let output_fn = dest.join(entry.url.as_str());
                println!("Writing {} to {:?}", entry.title, output_fn);

                fs::write(output_fn, rendered)?;
                // this is one of the latest posts, add it to the rss list
                if count == 0 {
                    rss_data.push(entry);
                }

                if let Some(tags) = &entry.tags {
                    // collect the tags for this post and associate them to the entry
                    for tag in tags.iter() {
                        let tag_entry = TagData {
                            url: entry.url.clone(),
                            title: entry.title.clone(),
                            tag: tag.to_string(),
                        };
                        match tag_map.get_mut(tag) {
                            Some(tl) => tl.push(tag_entry),
                            None => {
                                tag_map.insert(tag.to_string(), vec![tag_entry]);
                            }
                        };
                    }
                }
            }

            let index_data = IndexData {
                title: self.opts.title.clone(),
                entries: entry_set.to_vec(),
                pagination: pagination.clone(),
                published_at: now.into(),
                site_description: self
                    .opts
                    .description
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .to_string(),
                site_url: self.opts.url.clone(),
                domain: self.domain.to_string(),
                share_image: self.opts.share_image.clone(),
            };

            let index_fn = if count == 0 {
                let rss_fn = dest.join("index.rss");
                let rss_feed = self.hbs.render("atom", &index_data)?;
                println!("Writing RSS feed to {:?}", rss_fn);
                fs::write(rss_fn, rss_feed)?;
                "index.html".to_string()
            } else {
                format!("index{}.html", count)
            };

            let output_fn = dest.join(index_fn.as_str());
            let index_page = self.hbs.render("index", &index_data)?;
            println!("Writing page {} to {:?}", count, output_fn);
            fs::write(output_fn, index_page)?;
            count += 1;
        }

        // generate tag list
        let tags_data = json!({ "tags": tag_map });
        let tags_fn = dest.join("tags.html");
        let tags_page = self.hbs.render("tag-list", &tags_data)?;
        println!("Writing tags to {:?}", tags_fn);
        fs::write(tags_fn, tags_page)?;

        Ok(())
    }

    fn parse_entry(&self, file: &Path) -> Result<PageData> {
        let buf = fs::read_to_string(file)?;

        // extract metadata from post
        let mut sep_count = 0;
        let mut page_metadata = String::new();
        for line in buf.lines() {
            if line == HEADER_DELIMITER {
                sep_count += 1;
                if sep_count == 2 {
                    break;
                }
            }
            page_metadata.push_str(line);
            page_metadata.push('\n');
        }
        let page_metadata: PageMetadata = toml::from_str(&page_metadata)?;

        // generate the filename
        let page_filename = file.with_extension("").with_extension(".html");
        let page_filename = page_filename
            .to_str()
            .ok_or(BuilderError::BadFilename(Box::new(file.to_path_buf())))?;

        // render to html
        let mut comrak_options = ComrakOptions::default();
        comrak_options.render.unsafe_ = true;
        comrak_options.parse.smart = true;
        comrak_options.extension.front_matter_delimiter = Some(HEADER_DELIMITER.to_string());
        comrak_options.extension.strikethrough = true;
        comrak_options.extension.tagfilter = false;

        let contents = markdown_to_html(&buf, &comrak_options);
        let raw_text = strip_tags(&contents);
        let author = match page_metadata.author {
            Some(author) => Some(author),
            None => self.opts.author.clone(),
        };
        let now = Local::now();

        let truncate_len = self.opts.truncate.unwrap_or(DEFAULT_TRUNCATED);
        let truncated_text = truncate_text(&raw_text, truncate_len.try_into()?);
        println!("Parsed {:?} as {}", file, page_metadata.title);
        let entry = PageData {
            rendered_at: now.into(),
            created_at: page_metadata.date,
            raw_text: raw_text.clone(),
            contents,
            tags: page_metadata.tag_list,
            title: page_metadata.title,
            url: page_filename.to_string(),
            hero_image: page_metadata.hero_image,
            share_image: page_metadata.share_image,
            description: page_metadata.description,
            site_url: self.opts.url.clone(),
            domain: self.domain.clone(),
            author,
            year: now.format("%Y").to_string(),
            site_description: self.opts.description.clone(),
            truncated_contents: truncated_text.to_string(),
        };
        Ok(entry)
    }
}
