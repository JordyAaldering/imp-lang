use clap::Parser;
use imp_lang::Options;

fn main() {
    env_logger::init();
    let options = Options::parse();
    imp_lang::compile(options);
}
