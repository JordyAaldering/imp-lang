mod program;
mod fundef;
mod stmt;
mod expr;
mod tensor;
mod binary;
mod unary;
mod avis;
mod typ;

pub use program::*;
pub use fundef::*;
pub use stmt::*;
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

pub trait AstConfig: Clone + Copy + fmt::Debug {
    type ValueType: Clone + fmt::Debug + fmt::Display;
}

///
/// Untyped AST
///
#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = MaybeType;
}

///
/// Typed AST
///
#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}
