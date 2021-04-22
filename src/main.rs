mod builder;
mod helpers;
pub use crate::builder::Builder;

use clap::{Arg, App};

fn main() {
    let matches = App::new("What's cookin'")
        .version("1.0")
        .author("Todd Kennedy <todd@selfassembled.org>")
        .about("Static site blog generator")
        .arg(Arg::with_name("src")
            .index(1)
            .required(true)
            .value_name("SRC_DIR")
            .help("Sets the path where your markdown files live")
            .takes_value(true))
        .arg(Arg::with_name("dest")
            .index(2)
            .required(true)
            .value_name("DEST_DIR")
            .help("Sets the destination path for html files")
            .takes_value(true))
        .arg(Arg::with_name("entries")
            .short("e")
            .long("entries")
            .value_name("NUM_ENTRIES")
            .help("How many entries on the home page")
            .takes_value(true)
            .default_value("20"))
        .arg(Arg::with_name("template_dir")
            .short("t")
            .long("template_dir")
            .value_name("TEMPLATE_DIR")
            .help("Location of page templates")
            .takes_value(true)
            .default_value("templates"))
        .get_matches();

    let src_dir = matches.value_of("src").unwrap();
    let dest_dir = matches.value_of("dest").unwrap();
    let template_dir = matches.value_of("template_dir").unwrap();
    let count = matches.value_of("entries").unwrap().parse().unwrap();
    let mut b = Builder::new(src_dir, dest_dir, template_dir, count).unwrap();
    match b.build() {
        Ok(_a) => println!("Blog built!"),
        Err(e) => println!("{:?}", e),
    };
}
