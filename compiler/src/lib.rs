#![feature(associated_type_defaults)]

pub mod ast;
pub mod traverse;
pub mod show;
// Compiler phases
pub mod scp;
pub mod pre;
pub mod tc;
pub mod opt;
pub mod cg;

pub use crate::traverse::{Rewrite, Traverse, Visit};

use crate::ast::*;

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scp::scanparse(&src).unwrap();
    let ast = pre::flatten(ast);
    let ast = pre::to_ssa(ast);
    let ast = tc::type_infer(ast).unwrap();
    let ast = opt::constant_fold(ast);
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
