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

pub fn compile(breakpoint: Option<Phase>, infile: &PathBuf, outdir: Option<&PathBuf>) -> Option<PathBuf> {
    let src = fs::read_to_string(&infile).unwrap();
    if matches!(breakpoint, Some(Phase::RD)) {
        println!("{}", src.trim_end_matches('\n'));
        return None;
    }

    let parsed_arenas = Scope::<ParsedAst>::new();
    let mut ast = scp::scanparse(&src, &parsed_arenas).unwrap();
    if matches!(breakpoint, Some(Phase::SCP)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    let mut ast = tp::check_tp(ast).unwrap();
    if matches!(breakpoint, Some(Phase::CTP)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    tp::analyse_tp(&mut ast, &parsed_arenas);
    if matches!(breakpoint, Some(Phase::ATP)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    pre::flatten(&mut ast, &parsed_arenas);
    if matches!(breakpoint, Some(Phase::FLT)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    let untyped_arenas = Scope::<UntypedAst>::new();
    let mut ast = pre::to_ssa(ast, &untyped_arenas);
    if matches!(breakpoint, Some(Phase::SSA)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    tc::type_infer(&mut ast).unwrap();
    if matches!(breakpoint, Some(Phase::TI)) {
        let mut ast = ast;
        print!("{}", show::show(&mut ast));
        return None;
    }

    let typed_arenas = Scope::<TypedAst>::new();
    let mut ast = tc::resolve_dispatch(ast, &typed_arenas).unwrap();
    if matches!(breakpoint, Some(Phase::DR)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    opt::constant_fold(&mut ast);
    if matches!(breakpoint, Some(Phase::CF)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    opt::dead_code_removal(&mut ast);
    if matches!(breakpoint, Some(Phase::DCR)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    cg::rename_fundefs(&mut ast);
    if matches!(breakpoint, Some(Phase::RNF)) {
        print!("{}", show::show(&mut ast));
        return None;
    }

    let h_str = cg::emit_h(&mut ast);
    if matches!(breakpoint, Some(Phase::CGH)) {
        print!("{}", h_str);
        return None;
    }

    let module_name = format!("IMP{}", infile.file_stem().unwrap().to_str().unwrap());
    let c_str = cg::emit_c(&mut ast, &module_name);
    if matches!(breakpoint, Some(Phase::CGC)) {
        print!("{}", c_str);
        return None;
    }

    let rs_str = cg::emit_ffi(&mut ast);
    if matches!(breakpoint, Some(Phase::CGRS)) {
        print!("{}", rs_str);
        return None;
    }

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
