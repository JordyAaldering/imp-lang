use compiler::*;

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();

    println!("=== scanparse ===");
    let ast = scp::scanparse(&src).unwrap();
    println!("{}", show::show(&ast));

    println!("=== flatten ===");
    let ast = pre::flatten(ast);
    println!("{}", show::show(&ast));

    println!("=== convert_to_ssa ===");
    let ast = pre::to_ssa(ast);
    println!("{}", show::show(&ast));

    println!("=== type_infer ===");
    let ast = tc::type_infer(ast).unwrap();
    println!("{}", show::show(&ast));

    println!("=== constant_fold ===");
    let ast = opt::constant_fold(ast);
    println!("{}", show::show(&ast));

    // println!("=== codegen_header ===");
    // let mut codegen = cg::codegen_ffi::CompileFfi::new();
    // codegen.visit_program(&ast);
    // print!("{}", codegen.finish());

    println!("=== codegen_c ===");
    let mut codegen = cg::codegen_c::CompileC::new();
    codegen.visit_program(&ast);
    print!("{}", codegen.finish());
}
