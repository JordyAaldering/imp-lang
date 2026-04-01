// Declarations
mod program;
mod fundef;
// Statements
mod stmt;
mod assign;
mod ret;
// Expressions
mod expr;
mod tensor;
mod binary;
mod unary;
// Terminals
mod id;
mod avis;
mod typ;

// Declarations
pub use program::*;
pub use fundef::*;
// Statements
pub use stmt::*;
pub use assign::*;
pub use ret::*;
// Expressions
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
// Terminals
pub use id::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

pub trait AstConfig: Clone + Copy + fmt::Debug {
    type ValueType: Clone + fmt::Debug + fmt::Display;
}

#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = MaybeType;
}

#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}
