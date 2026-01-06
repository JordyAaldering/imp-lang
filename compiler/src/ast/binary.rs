use std::fmt;

use crate::ast::AstConfig;

use super::ArgOrVar;

#[derive(Clone, Debug)]
pub struct Binary<Ast: AstConfig> {
    pub l: ArgOrVar<Ast>,
    pub r: ArgOrVar<Ast>,
    pub op: Bop,
}

#[derive(Clone, Copy, Debug)]
pub enum Bop {
    // Arithmetic
    Add, Sub, Mul, Div,
    // Comparison
    Eq, Ne, Lt, Le, Gt, Ge,
}

impl fmt::Display for Bop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Bop::*;
        write!(f, "{}", match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
        })
    }
}
