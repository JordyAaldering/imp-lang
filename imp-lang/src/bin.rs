use clap::Parser;
use imp_lang::Options;

fn main() {
    let options = Options::parse();
    imp_lang::compile(options);
}
