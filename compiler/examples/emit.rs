use compiler::*;

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();

    println!("=== scanparse ===");
    let ast = scanparse::scanparse(&src).unwrap();
    println!("{}", show::show(&ast));

    println!("=== flatten ===");
    let ast = flatten::flatten(ast);
    println!("{}", show::show(&ast));

    println!("=== convert_to_ssa ===");
    let ast = convert_to_ssa::convert_to_ssa(ast);
    println!("{}", show::show(&ast));

    println!("=== type_infer ===");
    let ast = type_infer::type_infer(ast).unwrap();
    println!("{}", show::show(&ast));

    println!("=== codegen_header ===");
    let mut codegen = compile::codegen_ffi::CompileFfi::new();
    codegen.visit_program(&ast);
    print!("{}", codegen.finish());

    println!("=== codegen_c ===");
    let mut codegen = compile::codegen_c::CompileC::new();
    codegen.visit_program(&ast);
    print!("{}", codegen.finish());
}
