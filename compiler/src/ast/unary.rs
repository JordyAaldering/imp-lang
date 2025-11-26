use std::fmt;

use super::ArgOrVar;

#[derive(Clone, Debug)]
pub struct Unary {
    pub r: ArgOrVar,
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
