pub mod ast;
pub mod codegen_c;
pub mod codegen_header;
pub mod codegen_llvm;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod traverse;
pub mod type_infer;

use std::{ffi::CString, ptr};

use llvm_sys::core::LLVMPrintModuleToFile;

use crate::{ast::*, traverse::{Rewriter, Traversal}};

pub fn compile(src: &str) -> Program<TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(ast).unwrap();
    let ast = type_infer::TypeInfer::new().trav_program(ast).unwrap();
    ast
}

pub fn emit_header(ast: &Program<TypedAst>, outfile: &str) {
    let mut writer = codegen_header::CompileHeader::new();
    writer.trav_program(ast.clone()).unwrap();
    std::fs::write(outfile, writer.header).unwrap();
}

pub fn emit_llvm(ast: &Program<TypedAst>, outfile: &str) {
    // Just do the first fundef for now
    let ast = &ast.fundefs[0];

    unsafe {
        let cg = codegen_llvm::CodegenContext::new("my_module");
        cg.compile_fundef(ast);
        let err = ptr::null_mut();
        LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
    }
}

pub fn emit_c(ast: &Program<TypedAst>, outfile: &str) {
    let c_code = codegen_c::CodegenContext::new().compile_program(&ast);
    std::fs::write(outfile, c_code).unwrap();
}
