mod trav_name;
mod ast;
mod trav;
mod show;
mod scp;
mod tp;
mod pre;
mod tc;
mod opt;
mod cg;

use std::{fs, path::PathBuf};

use clap::{Parser, ValueEnum};

pub fn compile(options: Options) {
    let src = fs::read_to_string(&options.infile).unwrap();
    if matches!(options.b, Some(Phase::RD)) {
        println!("{}", src.trim_end_matches('\n'));
        return;
    }

    let mut ast = scp::scanparse(&src).unwrap();
    if matches!(options.b, Some(Phase::SCP)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    let mut ast = tp::check_tp(ast).unwrap();
    if matches!(options.b, Some(Phase::CTP)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    tp::analyse_tp(&mut ast);
    if matches!(options.b, Some(Phase::ATP)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    pre::flatten(&mut ast);
    if matches!(options.b, Some(Phase::FLT)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    let mut ast = pre::to_ssa(ast);
    if matches!(options.b, Some(Phase::SSA)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    tc::type_infer(&mut ast).unwrap();
    if matches!(options.b, Some(Phase::TI)) {
        let mut ast = ast;
        print!("{}", show::show(&mut ast));
        return;
    }

    let mut ast = tc::resolve_dispatch(ast).unwrap();
    if matches!(options.b, Some(Phase::DR)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    opt::constant_fold(&mut ast);
    if matches!(options.b, Some(Phase::CF)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    opt::dead_code_removal(&mut ast);
    if matches!(options.b, Some(Phase::DCR)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    cg::rename_fundefs(&mut ast);
    if matches!(options.b, Some(Phase::RNF)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    let c_str = cg::emit_c(&mut ast, options.module_name());
    if matches!(options.b, Some(Phase::CGC)) {
        print!("{}", c_str);
        return;
    }

    let h_str = cg::emit_h(&mut ast);
    if matches!(options.b, Some(Phase::CGH)) {
        print!("{}", h_str);
        return;
    }

    let rs_str = cg::emit_ffi(&mut ast);
    if matches!(options.b, Some(Phase::CGRS)) {
        print!("{}", rs_str);
        return;
    }

    if let Some(c_path) = options.c_path() {
        let h_path = options.h_path().unwrap();
        let rs_path = options.rs_path().unwrap();
        fs::write(c_path, c_str).unwrap();
        fs::write(h_path, h_str).unwrap();
        fs::write(rs_path, rs_str).unwrap();
    }
}

#[derive(Parser)]
#[derive(Default)]
pub struct Options {
    #[arg(short('b'), long("break"))]
    pub b: Option<Phase>,

    #[arg(short('o'), long("out"))]
    pub outdir: Option<PathBuf>,

    pub infile: PathBuf,
}

impl Options {
    pub fn new(infile: PathBuf, outdir: PathBuf) -> Self {
        Self {
            infile,
            outdir: Some(outdir),
            ..Default::default()
        }
    }

    pub fn module_name(&self) -> String {
        format!("IMP{}", self.infile.file_stem().unwrap().to_str().unwrap())
    }

    pub fn c_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("c")
        })
    }

    pub fn h_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("h")
        })
    }

    pub fn rs_path(&self) -> Option<PathBuf> {
        self.outdir.as_ref().map(|outdir| {
            outdir.join(self.module_name()).with_extension("rs")
        })
    }
}

#[derive(ValueEnum)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Read input
    RD,
    /// Scanning/parsing
    SCP,
    /// Check type pattern
    CTP,
    /// Analyse type pattern
    ATP,
    /// Flatten
    FLT,
    /// Convert to SSA
    SSA,
    /// Type inference
    TI,
    /// Function dispatch resolution
    DR,
    /// Constant folding
    CF,
    /// Dead code removal
    DCR,
    /// Rename fundefs
    RNF,
    /// C code generation
    CGC,
    /// C header code generation
    CGH,
    /// Rust FFI code generation
    CGRS,
}
