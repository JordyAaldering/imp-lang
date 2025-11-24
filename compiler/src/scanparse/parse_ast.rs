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

#[derive(Copy, Clone, Debug)]
pub enum Bop {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Copy, Clone, Debug)]
pub enum Uop {
    Neg, Not,
}

#[derive(Copy, Clone, Debug)]
pub enum Type {
    U32,
    Bool,
}
