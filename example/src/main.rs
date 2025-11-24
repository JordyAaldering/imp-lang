use std::{env, fs};

use compiler::*;

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let ast = scanparse::parse(&src).unwrap();
    println!("{:?}", ast);
}
