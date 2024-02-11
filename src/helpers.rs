use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, FixedOffset, Local};
use color_eyre::Result;

pub fn parse_date(date: &str) -> DateTime<FixedOffset> {
    match DateTime::parse_from_rfc3339(date) {
        Ok(d) => d,
        Err(_e) => {
            println!("Unable to parse {} as a date", date);
            DateTime::<FixedOffset>::from(Local::now())
        }
    }
}

pub fn get_entries(src: &Path) -> Result<Vec<PathBuf>> {
    let mut entries: Vec<_> = vec![];
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            entries.push(entry.path());
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATE_FORMAT: &str = "%A, %b %e, %Y";

    #[test]
    fn parses_rfc3339_input() {
        let date = "2021-05-07T00:00:00-07:00";
        let parsed = parse_date(date);
        let display = parsed.format(DATE_FORMAT).to_string();
        assert_eq!(display, "Friday, May  7, 2021");
    }

    #[test]
    fn returns_current_time_on_bad_input() {
        let date = "Wednesday, May  8, 2021";
        let local = DateTime::<FixedOffset>::from(Local::now());
        let parsed = parse_date(date);
        let display_local = local.format(DATE_FORMAT).to_string();
        let display_parsed = parsed.format(DATE_FORMAT).to_string();
        assert_ne!(date, display_parsed);
        assert_eq!(display_local, display_parsed);
    }

    #[test]
    fn reads_only_files() -> Result<()> {
        let mut fixtures = PathBuf::new();
        fixtures.push("fixtures/data");
        let entries = get_entries(&fixtures)?;
        assert_eq!(entries.len(), 3);
        Ok(())
    }
}
