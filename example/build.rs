use std::fs;
use std::path::Path;

use compiler::*;

// fn build_llvm() {
//     let out_dir = std::env::var("OUT_DIR").unwrap();
//     let h_path = Path::new(&out_dir).join("simple.rs");
//     let dst_ll = Path::new(&out_dir).join("simple.ll");
//     let dst_o = Path::new(&out_dir).join("simple.o");
//     let bc_path = dst_ll.with_extension("bc");

//     let src = fs::read_to_string(&"src/simple.imp").unwrap();
//     let ast = compile(&src);
//     emit_header(&ast, h_path.to_str().unwrap());
//     emit_llvm(&ast, dst_ll.to_str().unwrap());

//     Command::new("llvm-as")
//         .args([dst_ll.to_str().unwrap()])
//         .status()
//         .expect("failed to assemble LLVM");

//     Command::new("llc")
//         .args([bc_path.to_str().unwrap(), "-filetype=obj", "-o", dst_o.to_str().unwrap()])
//         .status()
//         .expect("failed to generate object file");

//     println!("cargo:rerun-if-changed=src/simple.dsl");
//     println!("cargo:rustc-link-search=native={}", out_dir);
//     println!("cargo:rustc-link-arg={}/simple.o", out_dir);
// }

fn build_c() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let c_path = Path::new(&out_dir).join("simple.c");
    let h_path = Path::new(&out_dir).join("simple.rs");

    let src = fs::read_to_string(&"src/simple.imp").unwrap();
    let ast = compile(&src);
    emit_header(&ast, h_path.to_str().unwrap());
    emit_c(&ast, c_path.to_str().unwrap());

    cc::Build::new()
        .file(&c_path)
        .compile("simple");

    println!("cargo:rerun-if-changed=src/simple.dsl");
}

fn main() {
    build_c();
}
