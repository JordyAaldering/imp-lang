use super::*;

/// The resolved dispatch target of a typed function call.
///
/// Direct overloading is disallowed, so a typed free-function call points to one concrete definition.
#[derive(Clone, Debug)]
pub enum CallTarget<'ast> {
    Function(&'ast Fundef<'ast, TypedAst>),
    TraitMethod {
        trait_name: String,
        method_name: String,
    },
}

impl<'ast> CallTarget<'ast> {
    pub fn name(&self) -> String {
        match self {
            CallTarget::Function(f) => f.name.clone(),
            CallTarget::TraitMethod { trait_name, method_name } => {
                format!("{trait_name}::{method_name}")
            }
        }
    }
}
