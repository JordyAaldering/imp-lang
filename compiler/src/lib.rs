pub mod ast;
pub mod compile;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod traverse;
pub mod type_infer;
pub mod undo_ssa;

// use std::{ffi::CString, ptr};

// use llvm_sys::core::LLVMPrintModuleToFile;

use crate::{ast::*, traverse::AstPass};

pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::convert_to_ssa(ast);
    let ast = type_infer::type_infer(ast).unwrap();
    ast
}

pub fn emit_header(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_header::CompileHeader::new();
    let _ = cg.pass_program(ast.clone()).unwrap();
    std::fs::write(outfile, cg.finish()).unwrap();
}

// pub fn emit_llvm(ast: &Program<'static, TypedAst>, outfile: &str) {
//     let ast = &ast.fundefs[0];
//     unsafe {
//         let cg = compile::codegen_llvm::CodegenContext::new("my_module");
//         cg.compile_fundef(ast);
//         let err = ptr::null_mut();
//         LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
//     }
// }

pub fn emit_c(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_c::CodegenContext::new();
    let _ = cg.pass_program(ast.clone()).unwrap();
    std::fs::write(outfile, cg.finish()).unwrap();
}
