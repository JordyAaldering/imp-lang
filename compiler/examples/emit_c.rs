use compiler::{traverse::Traverse, *};

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let ast = scanparse::scanparse(&src).unwrap();
    println!("{}", ast);
    let ast = convert_to_ssa::convert_to_ssa(ast);
    println!("{}", show::show(&ast));
    let mut ast = type_infer::type_infer(ast).unwrap();
    println!("{}", show::show(&ast));
    let c_code = compile::codegen_c::CodegenContext::new().trav_program(&mut ast);
    print!("{}", c_code);

    let undo_ssa = undo_ssa::UndoSsa::new().trav_program(&ast);
    println!("{:?}", undo_ssa);
}
