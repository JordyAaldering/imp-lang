use crate::ast::{Bop, Type, Uop};

#[derive(Debug)]
pub struct Program {
    pub fundefs: Vec<Fundef>,
}

#[derive(Debug)]
pub struct Fundef {
    pub id: String,
    pub args: Vec<(Type, String)>,
    pub ret_type: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    Assign {
        lhs: String,
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
}

#[derive(Debug)]
pub enum Expr {
    Tensor {
        iv: IndexVector,
        expr: Box<Expr>,
        lb: Box<Expr>,
        ub: Box<Expr>,
    },
    Binary {
        l: Box<Expr>,
        r: Box<Expr>,
        op: Bop,
    },
    Unary {
        r: Box<Expr>,
        op: Uop,
    },
    Identifier(String),
    Bool(bool),
    U32(u32),
}

#[derive(Debug)]
pub struct IndexVector(pub String);
