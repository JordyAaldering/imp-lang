use std::fs;
use std::process::Command;
use std::path::Path;

use compiler::*;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dst_ll = Path::new(&out_dir).join("simple.ll");
    let dst_o = Path::new(&out_dir).join("simple.o");
    let h_path = Path::new(&out_dir).join("simple.rs");

    println!("cargo:rerun-if-changed=src/simple.dsl");

    let src = fs::read_to_string(&"src/simple.imp").unwrap();
    let parse_ast = scanparse::parse(&src).unwrap();
    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(parse_ast).unwrap();
    let ast = type_infer::TypeInfer::new().infer_program(ast).unwrap();
    imp4llvm::compile(&ast.fundefs[0], dst_ll.to_str().unwrap());

    // 2. Convert LLVM IR → object file using llvm-as + llc
    Command::new("llvm-as")
        .args([dst_ll.to_str().unwrap()])
        .status()
        .expect("failed to assemble LLVM");

    let bc_path = dst_ll.with_extension("bc");

    Command::new("llc")
        .args([bc_path.to_str().unwrap(), "-filetype=obj", "-o", dst_o.to_str().unwrap()])
        .status()
        .expect("failed to generate object file");

    imp4llvm::compile_header(&ast.fundefs[0], h_path.to_str().unwrap());

    // 3. Tell Rust to link it
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-arg={}/simple.o", out_dir);
}
