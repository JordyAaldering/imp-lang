use compiler::{traverse::Rewriter, *};

use std::{env, fs};

fn main() {
    let file = env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file).unwrap();
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::ConvertToSsa::new().convert_program(ast).unwrap();
    show::Show::new().show_program(&ast);
    let ast = type_infer::TypeInfer::new().trav_program(ast).unwrap();
    show::Show::new().show_program(&ast);
    let c_code = compile::codegen_c::CodegenContext::new().compile_program(&ast);
    print!("{}", c_code);

    let undo_ssa = undo_ssa::UndoSsa::new().trav_program(&ast);
    println!("{:#?}", undo_ssa);
}
