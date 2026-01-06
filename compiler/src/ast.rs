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

pub trait AstConfig: Clone + Copy + fmt::Debug {
    type SlotKey: slotmap::Key;

    type ValueType: Clone + fmt::Debug + fmt::Display;
}

///
/// Untyped AST
///
#[derive(Clone, Copy, Debug)]
pub struct UntypedAst;

slotmap::new_key_type! { pub struct UntypedKey; }

impl AstConfig for UntypedAst {
    type SlotKey = UntypedKey;

    type ValueType = MaybeType;
}

///
/// Typed AST
///
#[derive(Clone, Copy, Debug)]
pub struct TypedAst;

slotmap::new_key_type! { pub struct TypedKey; }

impl AstConfig for TypedAst {
    type SlotKey = TypedKey;

    type ValueType = Type;
}
