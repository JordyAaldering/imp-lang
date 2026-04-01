use std::fmt;

use super::{Id, AstConfig};

#[derive(Clone, Debug)]
pub struct Unary<'ast, Ast: AstConfig> {
    pub r: Id<'ast, Ast>,
    pub op: Uop,
}

#[derive(Clone, Copy, Debug)]
pub enum Uop {
    Neg, Not,
}

impl fmt::Display for Uop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Uop::*;
        match self {
            Neg => write!(f, "-"),
            Not => write!(f, "!"),
        }
    }
}
