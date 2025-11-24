use super::{parse_ast as ast, parser::ParseError};

pub trait Operator {
    fn precedence(&self) -> usize;

    fn associativity(&self) -> Assoc;
}

#[derive(Clone, Copy)]
pub enum Assoc {
    /// Left-to-right associative (e.g. `+`)
    LeftToRight,
    /// Right-to-left associative (e.g. `^`)
    #[allow(unused)]
    RightToLeft,
    /// Non-associative (e.g. `>`)
    NonAssoc,
}

pub fn precedes(l: &Option<impl Operator>, r: &impl Operator) -> Result<bool, ParseError> {
    if let Some(l) = l {
        use Assoc::*;
        match (l.associativity(), r.associativity()) {
            (NonAssoc, NonAssoc) => Err(ParseError::NonAssociative),
            (_, RightToLeft) => Ok(l.precedence() <= r.precedence()),
            _ => Ok(l.precedence() < r.precedence()),
        }
    } else {
        // `l` is none; this is the first operator we are parsing
        Ok(true)
    }
}

impl Operator for ast::Bop {
    fn precedence(&self) -> usize {
        use ast::Bop::*;
        match self {
            Eq | Ne => 2,
            Lt | Le | Gt | Ge => 3,
            Add | Sub => 4,
            Mul | Div => 5,
        }
    }

    fn associativity(&self) -> Assoc {
        use ast::Bop::*;
        match self {
            Add | Sub | Mul | Div => Assoc::LeftToRight,
            Eq | Ne | Lt | Le | Gt | Ge => Assoc::NonAssoc,
        }
    }
}

impl Operator for ast::Uop {
    /// Unary operators always have precedence
    fn precedence(&self) -> usize {
        256
    }

    /// Unary operators are always left-to-right associative
    fn associativity(&self) -> Assoc {
        Assoc::LeftToRight
    }
}
