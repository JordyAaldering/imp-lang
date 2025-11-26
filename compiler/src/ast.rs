mod binary;
mod unary;

pub use binary::{Binary, Bop};
pub use unary::{Unary, Uop};

use std::ops::Index;

use slotmap::*;

new_key_type! { pub struct VarKey; }
new_key_type! { pub struct ExprKey; }

#[derive(Clone, Debug)]
pub struct Avis {
    pub _key: ArgOrVar,
    pub name: String,
    pub ty: Option<Type>,
}

impl Avis {
    pub fn new(key: ArgOrVar, name: &str, ty: Option<Type>) -> Self {
        Self { _key: key, name: name.to_owned(), ty }
    }

    pub fn set_type(&mut self, ty: Type) {
        assert!(self.ty.is_none());
        self.ty = Some(ty)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar {
    Arg(usize),
    Var(VarKey),
}

#[derive(Clone, Debug)]
pub struct Program {
    pub fundefs: Vec<Fundef>,
}

#[derive(Clone, Debug)]
pub struct Fundef {
    pub name: String,
    pub args: Vec<Avis>,
    pub vars: SlotMap<VarKey, Avis>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<VarKey, Expr>,
    pub ret_id: ArgOrVar,
}

impl Index<ArgOrVar> for Fundef {
    type Output = Avis;

    fn index(&self, x: ArgOrVar) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.vars[k],
        }
    }
}

impl Index<VarKey> for Fundef {
    type Output = Avis;

    fn index(&self, k: VarKey) -> &Self::Output {
        &self.vars[k]
    }
}

impl Fundef {
    pub fn nameof(&self, key: VarKey) -> &String {
        &self.vars[key].name
    }

    pub fn typof(&self, key: VarKey) -> &Option<Type> {
        &self.vars[key].ty
    }

    pub fn insert_var(&mut self, id: &str, ty: Option<Type>) -> VarKey {
        self.vars.insert_with_key(|key| Avis::new(ArgOrVar::Var(key), id, ty))
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Binary(Binary),
    Unary(Unary),
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
