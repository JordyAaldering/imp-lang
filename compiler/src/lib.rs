pub mod ast;
pub mod codegen_c;
pub mod codegen_header;
pub mod codegen_llvm;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod type_infer;
mod traverse;

use std::{ffi::CString, ptr};

use llvm_sys::core::LLVMPrintModuleToFile;

use ast::*;

pub fn compile(src: &str) -> Program {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(ast).unwrap();
    let ast = type_infer::TypeInfer::new().infer_program(ast).unwrap();
    ast
}

pub fn emit_header(ast: &Program, outfile: &str) {
    // Just do the first fundef for now
    let ast = &ast.fundefs[0];

    let header = codegen_header::compile_header(ast);
    std::fs::write(outfile, header).unwrap();
}

pub fn emit_llvm(ast: &Program, outfile: &str) {
    // Just do the first fundef for now
    let ast = &ast.fundefs[0];

    unsafe {
        let cg = codegen_llvm::CodegenContext::new("my_module");
        cg.compile_fundef(ast);
        let err = ptr::null_mut();
        LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
    }
}
