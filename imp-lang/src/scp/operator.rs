use super::{lexer::Token, parser::ParseError};

#[derive(Clone, Copy)]
pub(super) enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
}

impl Bop {
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Bop::Add => "@add",
            Bop::Sub => "@sub",
            Bop::Mul => "@mul",
            Bop::Div => "@div",
            Bop::Eq => "@eq",
            Bop::Ne => "@ne",
        }
    }
}

impl TryInto<Bop> for &Token {
    type Error = ();

    fn try_into(self) -> Result<Bop, Self::Error> {
        match self {
            Token::Add => Ok(Bop::Add),
            Token::Sub => Ok(Bop::Sub),
            Token::Mul => Ok(Bop::Mul),
            Token::Div => Ok(Bop::Div),
            Token::Eq => Ok(Bop::Eq),
            Token::Ne => Ok(Bop::Ne),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Uop {
    Neg,
    Not,
}

impl Uop {
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Uop::Neg => "@neg",
            Uop::Not => "@not",
        }
    }
}

impl TryInto<Uop> for &Token {
    type Error = ();

    fn try_into(self) -> Result<Uop, Self::Error> {
        match self {
            Token::Sub => Ok(Uop::Neg),
            Token::Not => Ok(Uop::Not),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Assoc {
    /// Left-to-right associative (e.g. `+`)
    LeftToRight,
    /// Right-to-left associative (e.g. `^`)
    #[allow(unused)]
    RightToLeft,
    /// Non-associative (e.g. `>`)
    NonAssoc,
}

pub(super) trait Operator {
    fn precedence(&self) -> usize;

    fn associativity(&self) -> Assoc;
}

impl Operator for Bop {
    fn precedence(&self) -> usize {
        use Bop::*;
        match self {
            Eq | Ne => 2,
            Add | Sub => 4,
            Mul | Div => 5,
        }
    }

    fn associativity(&self) -> Assoc {
        use Bop::*;
        match self {
            Add | Sub | Mul | Div => Assoc::LeftToRight,
            Eq | Ne => Assoc::NonAssoc,
        }
    }
}

impl Operator for Uop {
    /// Unary operators always have precedence
    fn precedence(&self) -> usize {
        256
    }

    /// Unary operators are always left-to-right associative
    fn associativity(&self) -> Assoc {
        Assoc::LeftToRight
    }
}

pub(super) fn precedes(l: &Option<impl Operator>, r: &impl Operator) -> Result<bool, ParseError> {
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
