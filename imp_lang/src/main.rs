use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[derive(Default)]
pub struct Options {
    #[arg(short('b'), long("break"))]
    pub b: Option<imp_lang::Phase>,

    #[arg(short('o'), long("out"))]
    pub outdir: Option<PathBuf>,

    pub infile: PathBuf,
}

fn main() {
    env_logger::init();

    let options = Options::parse();

    let cpath = imp_lang::compile(options.b, &options.infile, options.outdir.as_ref());
    if let Some(cpath) = cpath {
        println!("Output written to: {}", cpath.display());
    }
}
