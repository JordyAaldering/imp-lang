#![feature(associated_type_defaults)]

pub mod ast;
pub mod compile;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod traverse;
pub mod type_infer;
pub mod undo_ssa;

use crate::{ast::*, traverse::AstPass};

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::convert_to_ssa(ast);
    let ast = type_infer::type_infer(ast).unwrap();
    ast
}

pub fn emit_header(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_header::CompileHeader::new();
    let _ = cg.pass_program(ast.clone());
    std::fs::write(outfile, cg.finish()).unwrap();
}

pub fn emit_c(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_c::CodegenContext::new();
    let _ = cg.pass_program(ast.clone());
    std::fs::write(outfile, cg.finish()).unwrap();
}
