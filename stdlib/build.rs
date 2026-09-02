use std::{env, path::PathBuf};

fn main() {
    let infile = "src/stdlib.imp";

    let outdir = env::var("OUT_DIR").unwrap();
    let outdir = PathBuf::from(&outdir);

    let cpath = imp_lang::compile(None, &infile.into(), Some(&outdir)).unwrap();
    let opath = cpath.file_stem().unwrap().to_str().unwrap();

    cc::Build::new()
        .file(&cpath)
        .include(&outdir)
        .compile(opath);

    println!("cargo:rerun-if-changed={infile}");
}
