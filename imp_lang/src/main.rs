use clap::Parser;

fn main() {
    env_logger::init();

    let options = imp_lang::Options::parse();

    imp_lang::compile(options);
}
