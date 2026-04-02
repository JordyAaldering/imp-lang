#![feature(associated_type_defaults)]

mod core;
mod ast;
mod traverse;
pub mod show;
// Compiler phases
mod scp;
mod pre;
mod tc;
mod opt;
mod cg;

pub use crate::core::*;

use crate::{ast::*, traverse::*};

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scp::scanparse(&src).unwrap();
    let ast = pre::flatten(ast);
    let ast = pre::to_ssa(ast);
    let ast = tc::type_infer(ast).unwrap();
    let ast = opt::constant_fold(ast);
    let ast = opt::dead_code_removal(ast);
    ast
}

pub fn emit_ffi(ast: &Program<'static, TypedAst>, outfile: &str) {
    let mut cg = cg::codegen_ffi::CompileFfi::new();
    cg.visit_program(&ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}

pub fn emit_c(ast: &Program<'static, TypedAst>, outfile: &str) {
    let mut cg = cg::codegen_c::CompileC::new();
    cg.visit_program(ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}
