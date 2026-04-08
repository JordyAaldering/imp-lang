use super::*;

/// The resolved dispatch target of a function call.
#[derive(Clone, Debug)]
pub enum CallTarget<'ast, Ast: AstConfig> {
    Function(&'ast Fundef<'ast, Ast>),
}

impl<'ast, Ast: AstConfig> CallTarget<'ast, Ast> {
    pub fn name(&self) -> String {
        match self {
            CallTarget::Function(f) => f.name.clone(),
        }
    }
}
