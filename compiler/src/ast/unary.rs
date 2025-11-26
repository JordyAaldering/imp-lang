use std::fmt;

use super::VarKey;

#[derive(Clone, Debug)]
pub struct Unary {
    pub r: VarKey,
    pub op: Uop,
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
