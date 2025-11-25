use std::{env, fs};

use compiler::*;

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let parse_ast = scanparse::parse(&src).unwrap();

    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(parse_ast).unwrap();
    show::Show.show_program(&ast);
}
