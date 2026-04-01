use std::fs;
use std::path::Path;

use compiler::*;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let c_path = Path::new(&out_dir).join("simple.c");
    let h_path = Path::new(&out_dir).join("simple.rs");

    let src = fs::read_to_string(&"src/simple.imp").unwrap();
    let mut ast = compile(&src);
    emit_header(&mut ast, h_path.to_str().unwrap());
    emit_c(&mut ast, c_path.to_str().unwrap());

    cc::Build::new()
        .file(&c_path)
        .compile("simple");

    println!("cargo:rerun-if-changed=src/simple.imp");
}
