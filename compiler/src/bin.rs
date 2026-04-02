use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let ast = compiler::compile(&src);
    println!("{}", compiler::show::show(&ast));
}
