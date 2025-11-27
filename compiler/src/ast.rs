mod binary;
mod unary;

pub use binary::{Binary, Bop};
pub use unary::{Unary, Uop};

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
    Arg(usize),
    Var(Ast::VarKey),
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
    pub ret_id: ArgOrVar<Ast>,
}

impl<Ast: AstConfig> Index<ArgOrVar<Ast>> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar<Ast>) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.vars[k],
        }
    }
}

impl<Ast: AstConfig> IndexMut<ArgOrVar<Ast>> for Fundef<Ast> {
    fn index_mut(&mut self, x: ArgOrVar<Ast>) -> &mut Self::Output {
        match x {
            ArgOrVar::Arg(i) => &mut self.args[i],
            ArgOrVar::Var(k) => &mut self.vars[k],
        }
    }
}

impl<Ast: AstConfig> Fundef<Ast> {
    pub fn insert_var(&mut self, id: &str, ty: Ast::ValueType) -> Ast::VarKey {
        self.vars.insert_with_key(|key| Avis::new(ArgOrVar::Var(key), id, ty))
    }
}

#[derive(Clone, Debug)]
pub enum Expr<Ast: AstConfig> {
    Binary(Binary<Ast>),
    Unary(Unary<Ast>),
    // I don't think var is actually needed. During parsing we do still need such a construct because we lack context
    // (A slotmap does not even exist yet, everything is just identifiers that may or may not exist)
    // But afterwards it is redundant
    //Var(VarKey),
    // We might even be able to do the same thing for constants, if we include this in the type information instead
    // - maybe not actually, as a varkey should come with an ssa as well. But maybe a type is the field of Const does work
    // - or alternatively, a map of constants alongside the ssa map
    Bool(bool),
    U32(u32),
}

#[derive(Copy, Clone, Debug)]
pub enum Type {
    U32,
    Bool,
}
