#![feature(associated_type_defaults)]

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
        println!("{src}");
        return;
    }

    let ast = scp::scanparse(&src).unwrap();
    if matches!(options.b, Some(Phase::SCP)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = tp::check_tp(ast).unwrap();
    if matches!(options.b, Some(Phase::CTP)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = tp::analyse_tp(ast);
    if matches!(options.b, Some(Phase::ATP)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = pre::flatten(ast);
    if matches!(options.b, Some(Phase::FLT)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = pre::to_ssa(ast);
    if matches!(options.b, Some(Phase::SSA)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = tc::type_infer(ast).unwrap();
    if matches!(options.b, Some(Phase::TI)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = opt::constant_fold(ast);
    if matches!(options.b, Some(Phase::CF)) {
        println!("{}", show::show(&ast));
        return;
    }

    let ast = opt::dead_code_removal(ast);
    if matches!(options.b, Some(Phase::DCR)) {
        println!("{}", show::show(&ast));
        return;
    }

    let mut ast = cg::rename_fundefs(ast);
    if matches!(options.b, Some(Phase::RNF)) {
        println!("{}", show::show(&ast));
        return;
    }

    cg::emit_c(&mut ast, options.module_name(), options.c_path());
    cg::emit_h(&mut ast, options.h_path());
    cg::emit_ffi(&mut ast, options.rs_path());
}
