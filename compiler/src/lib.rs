mod arena;
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

use crate::{ast::*, traverse::{Rewriter, Traversal}};

pub fn compile(src: &str) -> Program<TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::convert_to_ssa(ast);
    let ast = type_infer::TypeInfer::new().trav_program(ast).unwrap();
    ast
}

pub fn emit_header(ast: &mut Program<TypedAst>, outfile: &str) {
    let res = compile::codegen_header::CompileHeader::new().trav_program(ast).unwrap();
    std::fs::write(outfile, res).unwrap();
}

// pub fn emit_llvm(ast: &Program<TypedAst>, outfile: &str) {
//     let ast = &ast.fundefs[0];
//     unsafe {
//         let cg = compile::codegen_llvm::CodegenContext::new("my_module");
//         cg.compile_fundef(ast);
//         let err = ptr::null_mut();
//         LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
//     }
// }

pub fn emit_c(ast: &Program<TypedAst>, outfile: &str) {
    let c_code = compile::codegen_c::CodegenContext::new().compile_program(&ast);
    std::fs::write(outfile, c_code).unwrap();
}
