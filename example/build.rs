use std::fs;
use std::path::Path;

use imp_lang::*;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let imp_file = "src/simple.imp";
    let stem = Path::new(imp_file).file_stem().unwrap().to_str().unwrap();

    let c_path = Path::new(&out_dir).join(format!("{stem}.c"));
    let h_c_path = Path::new(&out_dir).join(format!("{stem}.h"));
    let h_rs_path = Path::new(&out_dir).join(format!("{stem}.rs"));

    let src = fs::read_to_string(imp_file).unwrap();
    let mut ast = compile(&src);
    emit_h(&mut ast, h_c_path.to_str().unwrap());
    emit_ffi(&mut ast, h_rs_path.to_str().unwrap());
    emit_c(&mut ast, c_path.to_str().unwrap());

    cc::Build::new()
        .file(&c_path)
        .include(&out_dir)
        .compile(stem);

    println!("cargo:rerun-if-changed={imp_file}");
}
