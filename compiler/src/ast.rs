mod program;
mod fundef;
mod expr;
mod tensor;
mod binary;
mod unary;
mod avis;
mod typ;

pub use program::*;
pub use fundef::*;
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

pub trait AstConfig: Clone + fmt::Debug {
    type ValueType: Clone + fmt::Debug;
}

#[derive(Clone, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = Option<Type>;
}

#[derive(Clone, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}
