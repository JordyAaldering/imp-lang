mod options;
mod ast;
mod traverse;
mod show;
// Compiler phases
mod scp;
mod tp;
mod pre;
mod tc;
mod opt;
mod cg;

use std::fs;

use crate::traverse::*;

pub use crate::options::*;

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

    let mut ast = pre::flatten(ast);
    if matches!(options.b, Some(Phase::FLT)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    let mut ast = pre::to_ssa(ast);
    if matches!(options.b, Some(Phase::SSA)) {
        print!("{}", show::show(&mut ast));
        return;
    }

    let mut ast = tc::type_infer(ast).unwrap();
    if matches!(options.b, Some(Phase::TI)) {
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
