pub mod ast;
pub mod codegen_header;
pub mod codegen_llvm;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod type_infer;

use std::{ffi::CString, ptr};

use llvm_sys::core::LLVMPrintModuleToFile;

use crate::ast::*;

pub fn compile_llvm(ast: &Fundef<TypedAst>, outfile: &str) {
    unsafe {
        let cg = codegen_llvm::CodegenContext::new("my_module");
        cg.compile_fundef(ast);
        let err = ptr::null_mut();
        LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
    }
}

pub fn compile_header(ast: &Fundef<TypedAst>, outfile: &str) {
    let header = codegen_header::compile_header(ast);
    std::fs::write(outfile, header).unwrap();
}
