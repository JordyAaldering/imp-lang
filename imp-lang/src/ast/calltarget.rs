use super::*;
use std::fmt;
use std::cell::RefCell;

/// The resolved dispatch target of a function call.
#[derive(Clone)]
pub enum CallTarget<'ast, Ast: AstConfig> {
    Function(&'ast RefCell<Fundef<'ast, Ast>>),
}

impl<'ast, Ast: AstConfig> fmt::Debug for CallTarget<'ast, Ast> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallTarget::Function(fun) => f.debug_tuple("Function").field(&fun.borrow().name).finish(),
        }
    }
}

impl<'ast, Ast: AstConfig> CallTarget<'ast, Ast> {
    pub fn name(&self) -> String {
        match self {
            CallTarget::Function(f) => f.borrow().name.clone(),
        }
    }
}
