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
    let ast = imp_lang::compile(&src);
    println!("{}", imp_lang::show::show(&ast));
}
