mod binary;
mod unary;

pub use binary::{Binary, Bop};
pub use unary::{Unary, Uop};

use std::fmt;

use slotmap::*;

new_key_type! { pub struct VarKey; }
new_key_type! { pub struct ExprKey; }

pub trait AstConfig {
    type ValueType: Clone + fmt::Debug;
}

pub struct UntypedAst;

impl AstConfig for UntypedAst {
    type ValueType = Option<Type>;
}

pub struct TypedAst;

impl AstConfig for TypedAst {
    type ValueType = Type;
}

#[derive(Clone, Debug)]
pub struct VarInfo<Ast: AstConfig> {
    _key: VarKey,
    id: String,
    ty: Ast::ValueType,
}

impl<Ast: AstConfig> VarInfo<Ast> {
    pub fn new(key: VarKey, id: &str, ty: Ast::ValueType) -> Self {
        Self { _key: key, id: id.to_owned(), ty }
    }

    pub fn ty(&self) -> &Ast::ValueType {
        &self.ty
    }
}

impl VarInfo<UntypedAst> {
    pub fn set_type(&mut self, ty: Type) {
        assert!(self.ty.is_none());
        self.ty = Some(ty)
    }
}

#[derive(Clone, Debug)]
pub struct Program<Ast: AstConfig> {
    pub fundefs: Vec<Fundef<Ast>>,
}

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    pub id: String,
    /// ordered 'arena' containing a mapping of argument key to avis info
    ///
    /// Hmm. How can we create a unique varkey if we dont use a slotmap
    /// Perhaps we can use a vec here, turn vars into a secondary map, and then use a default slotmap to contain both
    pub args: Vec<VarKey>,
    //args: Vec<(VarKey, VarInfo)>,
    /// arena containing a mapping of variable keys to avis info
    /// If we look for a key but it does not exist here, it must be an arg
    /// TODO: I think currently args are still inserted here. Dont do this! Instead, make the args vec a vec of tuples <VarKey, VarInfo>
    ///     hmmm this is problematic though, because we need some sort of slotmap to generate keys
    ///     perhaps we need an enum: either a key for variables, or an index for arguments
    ///     but will that require lots of boilerplate every time?
    pub vars: SlotMap<VarKey, VarInfo<Ast>>,
    /// arena containing a mapping of variable keys to their ssa assignment expressions
    /// two options for multi-return:
    ///  1) also keep track of return index here
    ///  2) add tuple types, and insert extraction functions, then there is always only one lhs
    /// I am leaning towards option 1
    pub ssa: SecondaryMap<VarKey, Expr>,
    pub ret_value: VarKey,
}

impl<Ast: AstConfig> Fundef<Ast> {
    pub fn nameof(&self, key: VarKey) -> &String {
        &self.vars[key].id
    }

    pub fn typof(&self, key: VarKey) -> &Ast::ValueType {
        &self.vars[key].ty
    }

    pub fn insert_var(&mut self, id: &str, ty: Ast::ValueType) -> VarKey {
        self.vars.insert_with_key(|key| VarInfo::new(key, id, ty))
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
