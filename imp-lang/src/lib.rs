#![feature(associated_type_defaults)]

mod ast;
mod phase;
mod traverse;
mod show;
// Compiler phases
mod scp;
mod tp;
mod pre;
mod tc;
mod opt;
mod cg;

use std::path::Path;

use crate::{ast::*, traverse::*};

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scp::scanparse(src).unwrap();
    println!("{}", show::show(&ast));
    let ast = tp::analyse_tp(ast);
    println!("{}", show::show(&ast));
    let ast = pre::flatten(ast);
    println!("{}", show::show(&ast));
    let ast = pre::to_ssa(ast);
    println!("{}", show::show(&ast));
    let ast = tc::type_infer(ast).unwrap();
    println!("{}", show::show(&ast));
    let ast = opt::constant_fold(ast);
    println!("{}", show::show(&ast));
    let ast = opt::dead_code_removal(ast);
    println!("{}", show::show(&ast));
    ast
}

pub fn rename_fundefs(ast: &mut Program<'static, TypedAst>) {
    cg::rename_fundefs::rename_fundefs(ast);
}

pub fn emit_ffi(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    rename_fundefs(ast);
    let mut cg = cg::codegen_ffi::CompileFfi::new();
    cg.visit_program(ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}

pub fn emit_c(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    rename_fundefs(ast);
    let stem = Path::new(outfile).file_stem().unwrap().to_str().unwrap().to_owned();
    let mut cg = cg::codegen_c::CompileC::new(&stem);
    cg.visit_program(ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}

pub fn emit_h(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    rename_fundefs(ast);
    let mut cg = cg::codegen_h::CompileH::new();
    cg.visit_program(ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}
