use chrono::{DateTime, FixedOffset, Local};
use std::fs::read_dir;
use std::io;
use std::path::PathBuf;

pub fn parse_date(date: &str) -> DateTime<FixedOffset> {
    match DateTime::parse_from_rfc3339(&date) {
        Ok(d) => d,
        Err(_e) => {
            println!("Unable to parse {} as a date", date);
            DateTime::<FixedOffset>::from(Local::now())
        }
    }
}

pub fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(',').map(|e| String::from(e.trim())).collect()
}

pub fn get_entries(src: &str) -> io::Result<Vec<PathBuf>> {
    let entries = read_dir(src)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    Ok(entries)
}
