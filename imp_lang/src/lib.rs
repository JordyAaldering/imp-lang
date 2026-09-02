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

use crate::ast::{Scope, ParsedAst, TypedAst, UntypedAst};

macro_rules! breakpoint {
    ($breakpoint:ident, $phase:pat, $ast:ident) => {
        if matches!($breakpoint, Some($phase)) {
            print!("{}", show::show(&mut $ast));
            return None;
        }
    };
}

macro_rules! breakpoint_str {
    ($breakpoint:ident, $phase:pat, $src:ident) => {
        if matches!($breakpoint, Some($phase)) {
            print!("{}", $src);
            return None;
        }
    };
}

pub fn compile(breakpoint: Option<Phase>, infile: &PathBuf, outdir: Option<&PathBuf>) -> Option<PathBuf> {
    let src = fs::read_to_string(&infile).unwrap();
    breakpoint_str!(breakpoint, Phase::RD, src);

    let parsed_scope = Scope::<ParsedAst>::new();
    let mut ast = scp::scanparse(&src, &parsed_scope).unwrap();
    breakpoint!(breakpoint, Phase::SCP, ast);

    let mut ast = tp::check_tp(ast).unwrap();
    breakpoint!(breakpoint, Phase::CTP, ast);

    tp::analyse_tp(&mut ast, &parsed_scope);
    breakpoint!(breakpoint, Phase::ATP, ast);

    pre::flatten(&mut ast, &parsed_scope);
    breakpoint!(breakpoint, Phase::FLT, ast);

    let untyped_scope = Scope::<UntypedAst>::new();
    let mut ast = pre::to_ssa(ast, &untyped_scope);
    breakpoint!(breakpoint, Phase::SSA, ast);

    tc::type_infer(&mut ast).unwrap();
    breakpoint!(breakpoint, Phase::TI, ast);

    let typed_scope = Scope::<TypedAst>::new();
    let mut ast = tc::resolve_dispatch(ast, &typed_scope).unwrap();
    breakpoint!(breakpoint, Phase::DR, ast);

    opt::constant_fold(&mut ast);
    breakpoint!(breakpoint, Phase::CF, ast);

    opt::dead_code_removal(&mut ast);
    breakpoint!(breakpoint, Phase::DCR, ast);

    cg::rename_fundefs(&mut ast);
    breakpoint!(breakpoint, Phase::RNF, ast);

    let h_str = cg::emit_h(&mut ast);
    breakpoint_str!(breakpoint, Phase::CGH, h_str);

    let module_name = format!("IMP{}", infile.file_stem().unwrap().to_str().unwrap());
    let c_str = cg::emit_c(&mut ast, &module_name);
    breakpoint_str!(breakpoint, Phase::CGC, c_str);

    let rs_str = cg::emit_ffi(&mut ast);
    breakpoint_str!(breakpoint, Phase::CGRS, rs_str);

    if let Some(outdir) = outdir {
        let c_path = outdir.join(&module_name).with_extension("c");
        let h_path = outdir.join(&module_name).with_extension("h");
        let rs_path = outdir.join(&module_name).with_extension("rs");

        fs::write(&c_path, c_str).unwrap();
        fs::write(&h_path, h_str).unwrap();
        fs::write(&rs_path, rs_str).unwrap();

        Some(c_path)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(clap::ValueEnum)]
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
    /// C header code generation
    CGH,
    /// C code generation
    CGC,
    /// Rust FFI code generation
    CGRS,
}
