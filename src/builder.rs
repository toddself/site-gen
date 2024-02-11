use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use chrono::{DateTime, FixedOffset, Local};
use color_eyre::Result;
use comrak::{markdown_to_html, ComrakOptions};
use handlebars::Handlebars;
use serde_json::{json, Value};
use thiserror::Error;
use truncate_string_at_whitespace::truncate_text;
use voca_rs::strip::strip_tags;

use crate::helpers::{get_entries, parse_date};
use crate::Opt;

#[derive(Debug)]
struct FileEntry {
    modified: DateTime<FixedOffset>,
    raw_text: String,
    contents: String,
    tags: Vec<String>,
    title: String,
    url: String,
    hero_image: Option<String>,
    share_image: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
pub struct Builder<'blog> {
    opts: Opt,
    files: Vec<PathBuf>,
    entries: Vec<FileEntry>,
    hbs: Handlebars<'blog>,
}

#[derive(Debug, Error)]
enum BuilderError {
    #[error("{0:?} contains invalid unicode identifiers")]
    BadFilename(Box<PathBuf>),

    #[error("Missing required value {0}")]
    MissingValue(String),

    #[error("URL had no host")]
    BadURL,
}

const HEADER_DELIMITER: &str = "---";
const DATE_FORMAT: &str = "%A, %b %e, %Y";

impl<'blog> Builder<'blog> {
    pub fn new(opts: Opt) -> Result<Builder<'blog>> {
        let dest = &opts
            .dest
            .clone()
            .ok_or(BuilderError::MissingValue("dest".to_string()))?;
        fs::DirBuilder::new().recursive(true).create(dest)?;

        let src = &opts
            .src
            .clone()
            .ok_or(BuilderError::MissingValue("src".to_string()))?;
        let src = PathBuf::from(src);
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

        Ok(Builder {
            opts,
            files,
            entries: vec![],
            hbs,
        })
    }

    pub fn build(&mut self) -> Result<()> {
        for file in self.files.iter() {
            let entry = self.parse_entry(file)?;
            self.entries.push(entry);
        }

        self.entries.sort_by(|a, b| {
            let bd = b.modified.signed_duration_since(a.modified);
            let ad = a.modified.signed_duration_since(b.modified);
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
                    0 => json!({
                        "name": "home",
                        "url": "index.html",
                    }),
                    _ => json!({
                        "name": format!("page {}", index),
                        "url": format!("index{}.html", index),
                    }),
                });
            }
        }

        // generate the pages
        let now = Local::now();
        let mut rss_data: Vec<_> = vec![];
        let mut tag_map: BTreeMap<String, Vec<Value>> = BTreeMap::new();

        let dest = &self
            .opts
            .dest
            .clone()
            .ok_or(BuilderError::MissingValue("dest".to_string()))?;
        let dest = PathBuf::from(dest);

        let url = &self
            .opts
            .url
            .clone()
            .ok_or(BuilderError::MissingValue("url".to_string()))?;
        let domain = url::Url::parse(url)?;
        let domain = domain.host().ok_or(BuilderError::BadURL)?;

        for entry_set in self.entries.chunks(num_per_page.into()) {
            // output individual page, and add to rss and tag dictionaries
            for entry in entry_set {
                let entry_text = if let Some(trun_len) = &self.opts.truncate {
                    truncate_text(&entry.raw_text, *trun_len as usize)
                } else {
                    entry.raw_text.as_str()
                };
                let post_data = json!({
                    "title": entry.title,
                    "contents": entry.contents,
                    "tags": entry.tags,
                    "url": entry.url,
                    "modified": entry.modified.format(DATE_FORMAT).to_string(),
                    "hero_image": entry.hero_image,
                    "share_image": entry.share_image,
                    "description": entry.description.as_ref().unwrap_or(&truncate_text(&entry.raw_text, 300).to_string()),
                    "site_url": self.opts.url,
                });
                let rendered = self.hbs.render("entry", &post_data)?;
                let output_fn = dest.join(entry.url.as_str());
                println!("Writing {} to {:?}", entry.title, output_fn);
                fs::write(output_fn, rendered)?;

                // this is one of the latest posts, add it to the rss list
                if count == 0 {
                    rss_data.push(json!({
                        "title": entry.title,
                        "description": entry_text,
                        "modified": entry.modified.format("%+").to_string(),
                        "url": entry.url,
                        "site_url": &self.opts.url,
                        "contents": entry.contents,
                        "time_stamp": now.format("%+").to_string(),
                        "tag_date": now.format("%F").to_string(),
                        "author": &self.opts.author.clone().unwrap_or("anonymous".to_string()),
                        "domain": domain.to_string()
                    }));
                }

                // collect the tags for this post and associate them to the entry
                for tag in entry.tags.iter() {
                    let tag_entry = json!({
                        "url": entry.url,
                        "title": entry.title,
                        "tag": tag,
                    });
                    match tag_map.get_mut(tag) {
                        Some(tl) => tl.push(tag_entry),
                        None => {
                            tag_map.insert(tag.to_string(), vec![tag_entry]);
                        }
                    };
                }
            }

            // get whole chunk of posts to generate the paginated indexes
            let entries: Vec<_> = entry_set
                .iter()
                .map(|entry| {
                    json!({
                        "title": entry.title,
                        "contents": entry.contents,
                        "tags": entry.tags,
                        "url": entry.url,
                        "modified": entry.modified.format(DATE_FORMAT).to_string(),
                        "hero_image": entry.hero_image,
                        "site_url": &self.opts.url,
                    })
                })
                .collect();

            let page_data = json!({
                "title": &self.opts.title,
                "contents": entries,
                "pagination": pagination,
                "year": now.format("%Y").to_string(),
                "pub_date": now.format("%a, %e %b, %Y %T %Z").to_string(),
                "description": &self.opts.description,
                "site_url": self.opts.url,
            });

            let index_fn = if count == 0 {
                "index.html".to_string()
            } else {
                format!("index{}.html", count)
            };

            let output_fn = dest.join(index_fn.as_str());
            let index_page = self.hbs.render("index", &page_data)?;
            println!("Writing page {} to {:?}", count, output_fn);
            fs::write(output_fn, index_page)?;
            count += 1;
        }

        // generate rss with latest data
        let rss_data = json!({
            "title": &self.opts.title,
            "entries": rss_data,
            "year": now.format("%Y").to_string(),
            "pub_date": now.format("%a, %e %b, %Y %T %Z").to_string(),
            "site_url": self.opts.url,
            "description": &self.opts.description,
            "time_stamp": now.format("%+").to_string(),
            "tag_date": now.format("%F").to_string(),
            "domain": domain.to_string(),
        });
        let rss_fn = dest.join("index.rss");
        let rss_feed = self.hbs.render("atom", &rss_data)?;
        println!("Writing RSS feed to {:?}", rss_fn);
        fs::write(rss_fn, rss_feed)?;

        // generate tag list
        let tags_data = json!({ "tags": tag_map });
        let tags_fn = dest.join("tags.html");
        let tags_page = self.hbs.render("tag-list", &tags_data)?;
        println!("Writing tags to {:?}", tags_fn);
        fs::write(tags_fn, tags_page)?;

        Ok(())
    }

    fn parse_entry(&self, file: &Path) -> Result<FileEntry> {
        let filename = file
            .to_str()
            .ok_or(BuilderError::BadFilename(Box::new(file.to_owned())))?;
        let buf = fs::read_to_string(filename).unwrap();

        let mut pub_date = DateTime::<FixedOffset>::from(Local::now());
        let mut tag_list: Vec<String> = vec![];
        let mut title = String::new();
        let mut share_image = None;
        let mut hero_image = None;
        let mut description = None;

        // extract metadata from post
        let mut sep_count = 0;
        for line in buf.lines() {
            if line == HEADER_DELIMITER {
                sep_count += 1;
                if sep_count == 2 {
                    break;
                }
            }

            let elements: Vec<&str> = line.split(' ').collect();
            let data_type = elements.first();
            let data_value = elements[1..].join(" ");

            match data_type {
                Some(&"date:") => {
                    pub_date = parse_date(data_value.as_str());
                }
                Some(&"tags:") => {
                    tag_list = data_value
                        .split(',')
                        .map(|e| String::from(e.trim()))
                        .collect()
                }
                Some(&"title:") => {
                    title = data_value;
                }
                Some(&"share_image:") => {
                    share_image = Some(data_value);
                }
                Some(&"hero_image:") => {
                    hero_image = Some(data_value);
                }
                Some(&"description:") => {
                    description = Some(data_value);
                }
                _ => (),
            }
        }

        // generate the filename
        let url = match file.iter().last() {
            Some(u) => match u.to_str() {
                Some(u) => String::from(u).replace(".md", ".html"),
                None => String::from(filename),
            },
            None => String::from(filename),
        };

        let mut comrak_options = ComrakOptions::default();
        comrak_options.render.unsafe_ = true;
        comrak_options.parse.smart = true;
        comrak_options.extension.front_matter_delimiter = Some(HEADER_DELIMITER.to_owned());
        comrak_options.extension.strikethrough = true;
        comrak_options.extension.tagfilter = false;
        let contents = markdown_to_html(buf.as_str(), &comrak_options);
        let raw_text = strip_tags(contents.as_str());

        println!("Parsed {:?} as {}", file, title);

        let entry = FileEntry {
            modified: pub_date,
            tags: tag_list,
            raw_text,
            contents,
            title,
            url,
            hero_image,
            share_image,
            description,
        };

        Ok(entry)
    }
}
