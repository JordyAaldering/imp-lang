use std::fmt;

use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Binary<'ast, Ast: AstConfig> {
    pub l: Ast::Operand<'ast>,
    pub r: Ast::Operand<'ast>,
    pub op: Bop,
}

#[derive(Clone, Copy, Debug)]
pub enum Bop {
    Add, Sub, Mul, Div, Lt, Le, Gt, Ge, Eq, Ne,
}

impl fmt::Display for Bop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Bop::*;
        match self {
            Add => write!(f, "+"),
            Sub => write!(f, "-"),
            Mul => write!(f, "*"),
            Div => write!(f, "/"),
            Lt => write!(f, "<"),
            Le => write!(f, "<="),
            Gt => write!(f, ">"),
            Ge => write!(f, ">="),
            Eq => write!(f, "=="),
            Ne => write!(f, "!="),
        }
    }
}
