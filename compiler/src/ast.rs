mod expr;
mod tensor;
mod binary;
mod unary;
mod typ;

pub use expr::Expr;
pub use tensor::{Tensor, IndexVector};
pub use binary::{Binary, Bop};
pub use unary::{Unary, Uop};
pub use typ::{Type, BaseType, Shape};

use std::{fmt, ops::{Index, IndexMut}};

use crate::arena::{Arena, Key, SecondaryArena};

pub trait AstConfig: Clone + fmt::Debug {
    type ValueType: Clone + fmt::Debug;
}

#[derive(Clone, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = Option<Type>;
}

#[derive(Clone, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
    pub _key: ArgOrVar,
}

impl<Ast: AstConfig> Avis<Ast> {
    pub fn new(key: ArgOrVar, name: &str, ty: Ast::ValueType) -> Self {
        Self { _key: key, name: name.to_owned(), ty }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(Key),
    /// Index vector
    Iv(Key),
}

#[derive(Clone, Debug)]
pub struct Program<Ast: AstConfig> {
    pub fundefs: Vec<Fundef<Ast>>,
}

impl<Ast: AstConfig> Program<Ast> {
    pub fn new() -> Self {
        Self {
            fundefs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    pub name: String,
    pub args: Vec<Avis<Ast>>,
    pub vars: Arena<Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryArena<Expr>,
    pub ret: ArgOrVar,
}

impl<Ast: AstConfig> Index<&ArgOrVar> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: &ArgOrVar) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[*i],
            ArgOrVar::Var(k) => &self.vars[*k],
            ArgOrVar::Iv(k) => &self.vars[*k],
        }
    }
}

impl<Ast: AstConfig> Index<ArgOrVar> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.vars[k],
            ArgOrVar::Iv(k) => &self.vars[k],
        }
    }
}

impl<Ast: AstConfig> IndexMut<ArgOrVar> for Fundef<Ast> {
    fn index_mut(&mut self, x: ArgOrVar) -> &mut Self::Output {
        match x {
            ArgOrVar::Arg(i) => &mut self.args[i],
            ArgOrVar::Var(k) => &mut self.vars[k],
            ArgOrVar::Iv(k) => &mut self.vars[k],
        }
    }
}
