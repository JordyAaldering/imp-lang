use std::path::PathBuf;

fn main() {
    let infile = "src/simple.imp";

    let outdir = std::env::var("OUT_DIR").unwrap();
    let outdir = PathBuf::from(&outdir);//.join("src");

    let options = imp_lang::Options::new(PathBuf::from(infile), outdir.clone());
    let cpath = options.c_path().unwrap();
    let opath = cpath.file_stem().unwrap().to_str().unwrap();

    imp_lang::compile(options);

    cc::Build::new()
        .file(&cpath)
        .include(&outdir)
        .compile(opath);

    println!("cargo:rerun-if-changed={infile}");
}
