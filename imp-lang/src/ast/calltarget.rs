use super::*;

/// The resolved dispatch target of a function call.
#[derive(Clone, Debug)]
pub enum CallTarget<'ast, Ast: AstConfig> {
    Function(&'ast Fundef<'ast, Ast>),
    TraitMethod {
        trait_name: String,
        method_name: String,
    },
}

impl<'ast, Ast: AstConfig> CallTarget<'ast, Ast> {
    pub fn name(&self) -> String {
        match self {
            CallTarget::Function(f) => f.name.clone(),
            CallTarget::TraitMethod { trait_name, method_name } => {
                format!("{trait_name}::{method_name}")
            }
        }
    }
}
