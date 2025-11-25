use compiler::*;
use imp4llvm::*;

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let parse_ast = scanparse::parse(&src).unwrap();

    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(parse_ast).unwrap();
    let ast = type_infer::TypeInfer::new().infer_program(ast).unwrap();

    unsafe {
        let cg = codegen::CodegenContext::new("my_module");
        cg.compile_fundef(&ast.fundefs[0]);
        llvm_sys::core::LLVMDumpModule(cg.module);
    }
}
