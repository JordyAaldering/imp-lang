use std::{fs, path::PathBuf};

use clap::Parser;

#[derive(Parser)]
struct Args {
    file: PathBuf,
}

fn main() {
    let Args {
        file,
    } = Args::parse();

    let src = fs::read_to_string(file).unwrap();
    let _ = imp_lang::compile(&src);
}
