use std::fmt;

use crate::visit::{Visit, Walk};

use super::{ArgOrVar, AstConfig};

#[derive(Clone, Debug)]
pub struct Binary<Ast: AstConfig> {
    pub l: ArgOrVar<Ast>,
    pub r: ArgOrVar<Ast>,
    pub op: Bop,
}

impl<Ast, W> Visit<Ast, W> for Binary<Ast>
where
    Ast: AstConfig,
    W: Walk<Ast>,
{
    fn visit(&self, walk: &mut W) -> W::Output {
        walk.trav_ssa(&self.l);
        walk.trav_ssa(&self.r);
        W::DEFAULT
    }
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
