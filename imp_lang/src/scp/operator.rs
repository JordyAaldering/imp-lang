use super::{lexer::Token, parser::ParseError};

pub(super) trait Operator: Copy {
    fn associativity(self) -> Associativity;

    fn precedence(self) -> usize;

    fn precedes(self, other: impl Operator) -> Result<bool, ParseError>;
}

#[derive(Clone, Copy)]
pub(super) enum Associativity {
    /// Left-to-right associative (e.g. `+`)
    LeftToRight,
    /// Right-to-left associative (e.g. `^`)
    #[allow(unused)]
    RightToLeft,
    /// Non-associative (e.g. `>`)
    NonAssoc,
}

#[derive(Clone, Copy)]
pub(super) struct PrecedenceFloor(pub usize);

#[derive(Clone, Copy)]
pub(super) enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Clone, Copy)]
pub(super) enum Uop {
    Neg,
    Not,
}

impl Bop {
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Bop::Add => "add",
            Bop::Sub => "sub",
            Bop::Mul => "mul",
            Bop::Div => "div",
            Bop::Lt => "lt",
            Bop::Le => "le",
            Bop::Gt => "gt",
            Bop::Ge => "ge",
            Bop::Eq => "eq",
            Bop::Ne => "ne",
        }
    }
}

impl Uop {
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Uop::Neg => "neg",
            Uop::Not => "not",
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
            Token::Lt => Ok(Bop::Lt),
            Token::Le => Ok(Bop::Le),
            Token::Gt => Ok(Bop::Gt),
            Token::Ge => Ok(Bop::Ge),
            Token::Eq => Ok(Bop::Eq),
            Token::Ne => Ok(Bop::Ne),
            _ => Err(()),
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

impl Operator for PrecedenceFloor {
    fn associativity(self) -> Associativity {
        Associativity::LeftToRight
    }

    fn precedence(self) -> usize {
        self.0
    }

    fn precedes(self, other: impl Operator) -> Result<bool, ParseError> {
        match other.associativity() {
            Associativity::RightToLeft => Ok(self.precedence() <= other.precedence()),
            _ => Ok(self.precedence() < other.precedence()),
        }
    }
}

impl Operator for Bop {
    fn associativity(self) -> Associativity {
        use Bop::*;
        match self {
            Add | Sub | Mul | Div => Associativity::LeftToRight,
            Lt | Le | Gt | Ge | Eq | Ne => Associativity::NonAssoc,
        }
    }

    fn precedence(self) -> usize {
        use Bop::*;
        match self {
            Lt | Le | Gt | Ge | Eq | Ne => 2,
            Add | Sub => 4,
            Mul | Div => 5,
        }
    }

    fn precedes(self, other: impl Operator) -> Result<bool, ParseError> {
        match (self.associativity(), other.associativity()) {
            (Associativity::NonAssoc, Associativity::NonAssoc) => Err(ParseError::NonAssociative),
            (_, Associativity::RightToLeft) => Ok(self.precedence() <= other.precedence()),
            _ => Ok(self.precedence() < other.precedence()),
        }
    }
}

impl Operator for Uop {
    /// Unary operators always have precedence
    fn precedence(self) -> usize {
        256
    }

    /// Unary operators are always left-to-right associative
    fn associativity(self) -> Associativity {
        Associativity::LeftToRight
    }

    fn precedes(self, other: impl Operator) -> Result<bool, ParseError> {
        match other.associativity() {
            Associativity::RightToLeft => Ok(self.precedence() <= other.precedence()),
            _ => Ok(self.precedence() < other.precedence()),
        }
    }
}
