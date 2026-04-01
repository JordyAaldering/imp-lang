#![feature(associated_type_defaults)]
//! IMP-lang compiler: transforms source code through SSA IR to typed code and compiled C.
//!
//! ## Architecture
//!
//! The compiler follows an ordered pipeline:
//! 1. **Scanning & Parsing** (`scanparse`) → `parse_ast::Program`
//! 2. **SSA Conversion** (`convert_to_ssa`) → `ast::Program<UntypedAst>`
//! 3. **Type Inference** (`type_infer`, `AstPass`) → `ast::Program<TypedAst>`
//! 4. **Code Generation** (C/Rust headers via `AstPass`)
//! 5. **Reverse Transform** (`undo_ssa`) → `compile_ast::Program` for output
//!
//! ## Key Design Decisions
//!
//! - **Borrowed Lifetime AST**: Uses `'ast` lifetime to ensure all AST nodes share
//!   a single allocation arena, simplifying memory management.
//! - **Ordered Function Bodies**: Functions are represented as ordered statement sequences
//!   (not purely functional DAGs), enabling future side effects like debug prints.
//! - **Uniform Traversal**: Core transformations use the `AstPass` trait for consistency.
//!   Cross-AST conversions (`convert_to_ssa`, `undo_ssa`) use manual traversal since
//!   `AstPass` targets same-type transforms.
//! - **Flexible Output Types**: `AstPass` uses associated type defaults, allowing passes
//!   to transform expression node types (e.g., constant folding).

pub mod ast;
pub mod compile;
pub mod convert_to_ssa;
pub mod scanparse;
pub mod show;
pub mod traverse;
pub mod type_infer;
pub mod undo_ssa;

use crate::{ast::*, traverse::AstPass};

/// Parse and fully compile source code.
///
/// Runs the complete pipeline: scanning → parsing → SSA conversion → type inference.
/// Returns a fully typed AST ready for code generation.
pub fn compile(src: &str) -> Program<'static, TypedAst> {
    let ast = scanparse::scanparse(&src).unwrap();
    let ast = convert_to_ssa::convert_to_ssa(ast);
    let ast = type_infer::type_infer(ast).unwrap();
    ast
}

/// Emit Rust FFI header bindings.
///
/// Runs the Rust header codegen pass, generating safe wrappers and unsafe FFI bindings.
pub fn emit_header(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_header::CompileHeader::new();
    let _ = cg.pass_program(ast.clone());
    std::fs::write(outfile, cg.finish()).unwrap();
}

/// Emit C99 code.
///
/// Runs the C codegen pass, generating compilable C99 code.
pub fn emit_c(ast: &mut Program<'static, TypedAst>, outfile: &str) {
    let mut cg = compile::codegen_c::CodegenContext::new();
    let _ = cg.pass_program(ast.clone());
    std::fs::write(outfile, cg.finish()).unwrap();
}
