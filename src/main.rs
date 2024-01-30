mod builder;
mod helpers;
use std::path::PathBuf;
use structopt::StructOpt;

pub use crate::builder::Builder;

#[derive(Debug, StructOpt)]
#[structopt(name = "blog-builder", about = "a poorly named static site generator")]
struct Opt {
    #[structopt(short, long, default_value = "20", help = "How many entries per page")]
    entries: usize,

    #[structopt(
        short,
        long,
        default_value = "templates",
        help = "Location of page templates"
    )]
    template_dir: PathBuf,

    #[structopt(
        parse(from_os_str),
        help = "Set the source path for your markdown files"
    )]
    src: PathBuf,

    #[structopt(parse(from_os_str), help = "Set the output directory")]
    dest: PathBuf,

    #[structopt(short, long)]
    title: String
}

fn main() {
    let opts = Opt::from_args();
    let mut b = Builder::new(&opts.src, &opts.dest, &opts.template_dir, opts.entries, opts.title).unwrap();
    match b.build() {
        Ok(_a) => println!("Blog built!"),
        Err(e) => println!("{:?}", e),
    };
}
