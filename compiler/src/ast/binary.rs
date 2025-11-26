use std::fmt;

use super::VarKey;

#[derive(Clone, Debug)]
pub struct Binary {
    pub l: VarKey,
    pub r: VarKey,
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
