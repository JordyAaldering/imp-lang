use compiler::*;

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let ast = compile(&src);
    let c_code = emit_c(&ast);
    println!("{}", c_code);
}
