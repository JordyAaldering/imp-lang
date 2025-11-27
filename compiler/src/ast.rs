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

use slotmap::*;

pub trait AstConfig: Clone + fmt::Debug {
    type VarKey: Key;

    type ValueType: Clone + fmt::Debug;
}

#[derive(Clone, Debug)]
pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type VarKey = UntypedKey;

    type ValueType = Option<Type>;
}

#[derive(Clone, Debug)]
pub struct TypedAst;

impl AstConfig for TypedAst {
    type VarKey = TypedKey;

    type ValueType = Type;
}

new_key_type! { pub struct TypedKey; }
new_key_type! { pub struct UntypedKey; }
new_key_type! { pub struct ExprKey; }

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
    pub _key: ArgOrVar<Ast>,
}

impl<Ast: AstConfig> Avis<Ast> {
    pub fn new(key: ArgOrVar<Ast>, name: &str, ty: Ast::ValueType) -> Self {
        Self { _key: key, name: name.to_owned(), ty }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar<Ast: AstConfig> {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(Ast::VarKey),
    /// Index vector
    IV(Ast::VarKey),
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
    pub vars: SlotMap<Ast::VarKey, Avis<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<Ast::VarKey, Expr<Ast>>,
    pub ret: ArgOrVar<Ast>,
}

impl<Ast: AstConfig> Index<&ArgOrVar<Ast>> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: &ArgOrVar<Ast>) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[*i],
            ArgOrVar::Var(k) => &self.vars[*k],
            ArgOrVar::IV(k) => &self.vars[*k],
        }
    }
}

impl<Ast: AstConfig> Index<ArgOrVar<Ast>> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar<Ast>) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.vars[k],
            ArgOrVar::IV(k) => &self.vars[k],
        }
    }
}

impl<Ast: AstConfig> IndexMut<ArgOrVar<Ast>> for Fundef<Ast> {
    fn index_mut(&mut self, x: ArgOrVar<Ast>) -> &mut Self::Output {
        match x {
            ArgOrVar::Arg(i) => &mut self.args[i],
            ArgOrVar::Var(k) => &mut self.vars[k],
            ArgOrVar::IV(k) => &mut self.vars[k],
        }
    }
}

impl<Ast: AstConfig> Fundef<Ast> {
    pub fn insert_var(&mut self, id: &str, ty: Ast::ValueType) -> Ast::VarKey {
        self.vars.insert_with_key(|key| Avis::new(ArgOrVar::Var(key), id, ty))
    }
}
