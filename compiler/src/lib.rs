#![feature(associated_type_defaults)]

pub mod ast;
pub mod compile;
pub mod flatten;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod traverse;
pub mod type_infer;

pub use crate::traverse::{Traverse, Visit};

use crate::ast::*;

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = flatten::flatten(ast);
    let ast = convert_to_ssa::convert_to_ssa(ast);
    let ast = type_infer::type_infer(ast).unwrap();
    ast
}

pub fn emit_ffi(ast: &Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_ffi::CompileFfi::new();
    cg.visit_program(&ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}

pub fn emit_c(ast: &Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_c::CompileC::new();
    cg.visit_program(ast);
    std::fs::write(outfile, cg.finish()).unwrap();
}
