use std::fmt;

use crate::visit::{Visit, Walk};

use super::{ArgOrVar, AstConfig};

#[derive(Clone, Debug)]
pub struct Unary<Ast: AstConfig> {
    pub r: ArgOrVar<Ast>,
    pub op: Uop,
}

impl<Ast, W> Visit<Ast, W> for Unary<Ast>
where
    Ast: AstConfig,
    W: Walk<Ast>,
{
    fn visit(&self, walk: &mut W) -> W::Output {
        walk.trav_ssa(&self.r);
        W::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Uop {
    Neg, Not,
}

impl fmt::Display for Uop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Uop::*;
        write!(f, "{}", match self {
            Neg => "-",
            Not => "!",
        })
    }
}
