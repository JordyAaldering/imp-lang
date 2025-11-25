pub mod codegen;
mod header;

use std::{ffi::CString, ptr};

use compiler::ast::*;
use llvm_sys::core::LLVMPrintModuleToFile;

pub fn compile(ast: &Fundef<TypedAst>, outfile: &str) {
    unsafe {
        let cg = codegen::CodegenContext::new("my_module");
        cg.compile_fundef(ast);
        let err = ptr::null_mut();
        LLVMPrintModuleToFile(cg.module, CString::new(outfile).unwrap().as_ptr(), err);
    }
}

pub fn compile_header(ast: &Fundef<TypedAst>, outfile: &str) {
    let header = header::compile_header(ast);
    std::fs::write(outfile, header).unwrap();
}
