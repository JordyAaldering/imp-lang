mod program;
mod fundef;
mod expr;
mod tensor;
mod binary;
mod unary;
mod avis;
mod typ;

pub use program::*;
pub use fundef::*;
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

use crate::arena::{Arena, Key, SecondaryArena};

pub trait Scoped<Ast: AstConfig, OutAst: AstConfig = Ast> {
    fn fargs(&self) -> &Vec<Avis<Ast>>;

    fn set_fargs(&mut self, fargs: Vec<Avis<Ast>>);

    fn pop_fargs(&mut self) -> Vec<Avis<OutAst>>;

    fn scopes(&self) -> &Vec<(Arena<Avis<Ast>>, SecondaryArena<Expr<Ast>>)>;

    fn push_scope(&mut self, ids: Arena<Avis<Ast>>, ssa: SecondaryArena<Expr<Ast>>);

    fn pop_scope(&mut self) -> (Arena<Avis<OutAst>>, SecondaryArena<Expr<OutAst>>);

    fn find_id(&self, key: ArgOrVar) -> Option<&Avis<Ast>> {
        match key {
            ArgOrVar::Arg(i) => self.fargs().get(i),
            ArgOrVar::Var(k) => self.find_key(k),
            ArgOrVar::Iv(k) => self.find_key(k),
        }
    }

    fn find_key(&self, key: Key) -> Option<&Avis<Ast>> {
        for (ids, _ssa) in self.scopes().iter().rev() {
            if let Some(avis) = ids.get(key) {
                return Some(avis)
            }
        }
        None
    }

    fn find_ssa(&self, key: Key) -> Option<&Expr<Ast>> {
        for (_ids, ssa) in self.scopes().iter().rev() {
            if let Some(expr) = ssa.get(key) {
                return Some(expr)
            }
        }
        None
    }

    fn depth(&self, key: Key) -> Option<usize> {
        for (depth, (ids, _ssa)) in self.scopes().iter().enumerate().rev() {
            if ids.get(key).is_some() {
                return Some(depth);
            }
        }
        None
    }
}

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
