use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use chrono::{DateTime, FixedOffset, Local};
use color_eyre::Result;
use comrak::{markdown_to_html, ComrakOptions};
use handlebars::Handlebars;
use serde_json::{json, Value};
use truncate_string_at_whitespace::truncate_text;
use voca_rs::strip::strip_tags;

use crate::helpers::{get_entries, parse_date};
use crate::Opt;

#[derive(Debug)]
pub struct FileEntry {
    pub modified: DateTime<FixedOffset>,
    pub filename: String,
    pub raw_text: String,
    pub contents: String,
    pub tags: Vec<String>,
    pub title: String,
    pub url: String,
}

#[derive(Debug)]
pub struct Builder<'blog> {
    opts: Opt,
    files: Vec<PathBuf>,
    entries: Vec<FileEntry>,
    hbs: Handlebars<'blog>,
}

const HEADER_DELIMITER: &str = "---";
const DATE_FORMAT: &str = "%A, %b %e, %Y";

impl<'blog> Builder<'blog> {
    pub fn new(opts: Opt) -> Result<Builder<'blog>> {
        match fs::DirBuilder::new().recursive(true).create(&opts.dest) {
            Ok(d) => d,
            Err(e) => return Err(e.into()),
        }

        let src = PathBuf::from(&opts.src);
        let files = match get_entries(&src) {
            Ok(f) => f,
            Err(_e) => vec![],
        };

        let mut hbs = Handlebars::new();
        let tmpl_src = PathBuf::from(&opts.template_dir);
        let templates: Vec<PathBuf> = match get_entries(&tmpl_src) {
            Ok(t) => t,
            Err(_e) => vec![],
        };

        for tpl_path in templates.iter() {
            let filename = tpl_path
                .to_str()
                .unwrap_or_else(|| panic!("Invalid unicode in filename: {:?}", tpl_path));

            let name = match tpl_path.iter().last() {
                Some(u) => match u.to_str() {
                    Some(u) => u.split('.').next().unwrap(),
                    None => filename,
                },
                None => filename,
            };
            hbs.register_template_file(name, tpl_path)?;
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
            let entry = self.parse_entry(file).unwrap();
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

        let now = Local::now();
        let mut rss_data: Vec<_> = vec![];
        let mut tag_map: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        let dest = PathBuf::from(&self.opts.dest);

        for entry_set in self.entries.chunks(num_per_page.into()) {
            for entry in entry_set {
                let post_data = json!({
                    "title": entry.title,
                    "contents": entry.contents,
                    "tags": entry.tags,
                    "url": entry.url,
                    "modified": entry.modified.format(DATE_FORMAT).to_string(),
                });
                let rendered = self.hbs.render("entry", &post_data)?;
                let output_fn = dest.join(entry.url.as_str());
                println!("Writing {} to {:?}", entry.title, output_fn);
                fs::write(output_fn, rendered)?;

                // this is one of the latest posts, add it to the rss list
                if count == 0 {
                    let entry_text = if let Some(trun_len) = &self.opts.truncate {
                        truncate_text(entry.raw_text.as_str(), *trun_len as usize)
                    } else {
                        entry.raw_text.as_str()
                    };
                    rss_data.push(json!({
                        "title": entry.title,
                        "description": entry_text,
                        "modified": entry.modified.format(DATE_FORMAT).to_string(),
                        "url": entry.url,
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
                            tag_map.insert(String::from(tag), vec![tag_entry]);
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
                    })
                })
                .collect();

            let page_data = json!({
                "title": &self.opts.title,
                "contents": entries,
                "pagination": pagination,
                "year": now.format("%Y").to_string(),
                "pub_date": now.format("%a, %e %b, %Y %T %Z").to_string(),
            });

            let index_fn = match count {
                0 => String::from("index.html"),
                _ => format!("index{}.html", count),
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
        });
        let rss_fn = dest.join("index.rss");
        let rss_feed = self.hbs.render("rss", &rss_data)?;
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

    fn parse_entry(&self, file: &Path) -> Result<FileEntry, std::io::Error> {
        let filename = file.to_str().unwrap();
        let buf = fs::read_to_string(filename).unwrap();

        let mut pub_date = DateTime::<FixedOffset>::from(Local::now());
        let mut tag_list: Vec<String> = vec![];
        let mut title = String::new();

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
                _ => (),
            }
        }

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
            filename: String::from(filename),
            tags: tag_list,
            raw_text,
            contents,
            title,
            url,
        };

        Ok(entry)
    }
}
