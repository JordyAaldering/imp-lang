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
    pub ret_expr: Expr,
}

#[derive(Debug)]
pub enum Stmt {
    Assign {
        lhs: String,
        expr: Expr,
    },
}

#[derive(Debug)]
pub enum Expr {
    Tensor {
        iv: String,
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

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for fundef in &self.fundefs {
            writeln!(f, "{}", fundef)?;
        }
        Ok(())
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

        writeln!(f, "    return {};", self.ret_expr)?;

        writeln!(f, "}}")
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Stmt::*;
        match self {
            Assign { lhs, expr } => {
                write!(f, "{} = {};", lhs, expr)
            },
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Expr::*;
        match self {
            Tensor { iv, expr, lb, ub } => {
                write!(f, "{{ {} | {} <= {} < {} }}", expr, lb, iv, ub)
            },
            Binary { l, r, op } => {
                write!(f, "({} {} {})", l, op, r)
            },
            Unary { r, op } => {
                write!(f, "{}{}", op, r)
            },
            // Terminals
            Identifier(v) => write!(f, "{}", v),
            Bool(v) => write!(f, "{}", v),
            U32(v) => write!(f, "{}", v),
        }
    }
}