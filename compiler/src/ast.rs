mod program;
mod fundef;
mod block;
mod expr;
mod tensor;
mod binary;
mod unary;
mod avis;
mod typ;

pub use program::*;
pub use fundef::*;
pub use block::*;
pub use expr::*;
pub use tensor::*;
pub use binary::*;
pub use unary::*;
pub use avis::*;
pub use typ::*;

use std::fmt;

use crate::arena::Key;

pub trait Scoped<Ast: AstConfig> {
    fn fargs(&self) -> &Vec<Avis<Ast>>;

    fn fargs_mut(&mut self) -> &mut Vec<Avis<Ast>>;

    fn scopes(&self) -> &Vec<Block<Ast>>;

    fn scopes_mut(&mut self) -> &mut Vec<Block<Ast>>;

    fn find_id(&self, key: ArgOrVar) -> Option<&Avis<Ast>> {
        match key {
            ArgOrVar::Arg(i) => self.fargs().get(i),
            ArgOrVar::Var(k) => self.find_key(k),
            ArgOrVar::Iv(k) => self.find_key(k),
        }
    }

    fn find_key(&self, key: Key) -> Option<&Avis<Ast>> {
        for scope in self.scopes().iter().rev() {
            if let Some(avis) = scope.ids.get(key) {
                return Some(avis)
            }
        }
        None
    }

    fn find_ssa(&self, key: Key) -> Option<&Expr<Ast>> {
        for scope in self.scopes().iter().rev() {
            if let Some(expr) = scope.ssa.get(key) {
                return Some(expr)
            }
        }
        None
    }

    fn depth(&self, key: Key) -> Option<usize> {
        for (depth, scope) in self.scopes().iter().enumerate().rev() {
            if scope.ids.get(key).is_some() {
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
