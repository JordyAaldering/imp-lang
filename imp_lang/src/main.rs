use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Options {
    /// Stop at the given [`imp_lang::Phase`].
    #[arg(short('b'), long("break"))]
    pub b: Option<imp_lang::Phase>,

    /// Print debug information for the given [`imp_lang::Phase`].
    ///
    /// (Currently not yet used)
    #[arg(short('d'), long("debug"), value_delimiter(','))]
    pub d: Vec<imp_lang::Phase>,

    #[arg(short('o'), long("out"))]
    pub outdir: Option<PathBuf>,

    pub infile: PathBuf,
}

fn init_logger(d: &[imp_lang::Phase]) {
    let mut builder = env_logger::Builder::new();

    // Note: this currently filters out ALL log messages other than those specified here, even ones that are not related to phases.
    for phase in d {
        builder.filter_module(phase.log_target(), log::LevelFilter::Debug);
    }

    builder.init();
}

fn main() {
    let options = Options::parse();

    init_logger(&options.d);

    let cpath = imp_lang::compile(options.b, &options.infile, options.outdir.as_ref());
    if let Some(cpath) = cpath {
        println!("Output written to: {}", cpath.display());
    }
}
