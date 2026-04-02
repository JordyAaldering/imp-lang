use std::fmt;

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
    Assign(Assign),
    Return(Return),
}

#[derive(Debug)]
pub struct Assign {
    pub lhs: String,
    pub expr: Expr,
}

#[derive(Debug)]
pub struct Return {
    pub expr: Expr,
}

#[derive(Debug)]
pub enum Expr {
    Tensor(Tensor),
    Binary(Binary),
    Unary(Unary),
    Id(String),
    Bool(bool),
    U32(u32),
}

#[derive(Debug)]
pub struct Tensor {
    pub iv: String,
    pub expr: Box<Expr>,
    pub lb: Box<Expr>,
    pub ub: Box<Expr>,
}

#[derive(Debug)]
pub struct Binary {
    pub l: Box<Expr>,
    pub r: Box<Expr>,
    pub op: Bop,
}

#[derive(Debug)]
pub struct Unary {
    pub r: Box<Expr>,
    pub op: Uop,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fundefs = self.fundefs.iter()
            .map(Fundef::to_string)
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", fundefs)
    }
}

impl fmt::Display for Fundef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = self.args.iter()
            .map(|(ty, id)| {
                format!("{} {}", ty, id)
            })
            .collect::<Vec<String>>()
            .join(", ");

        writeln!(f, "fn {}({}) -> {} {{", self.id, args, self.ret_type)?;

        for stmt in &self.body {
            writeln!(f, "    {}", stmt)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Assign(n) => write!(f, "{} = {};", n.lhs, n.expr),
            Stmt::Return(n) => write!(f, "return {};", n.expr),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Tensor(n) => write!(f, "{{ {} | {} <= {} < {} }}", n.expr, n.lb, n.iv, n.ub),
            Expr::Binary(n) => write!(f, "({} {} {})", n.l, n.op, n.r),
            Expr::Unary(n) => write!(f, "{}{}", n.op, n.r),
            Expr::Id(v) => write!(f, "{}", v),
            Expr::Bool(v) => write!(f, "{}", v),
            Expr::U32(v) => write!(f, "{}", v),
        }
    }
}
