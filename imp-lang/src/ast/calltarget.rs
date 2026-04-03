use super::*;

/// The resolved dispatch target of a typed function call.
///
/// Direct overloading is disallowed, so a typed free-function call points to one concrete definition.
#[derive(Clone, Debug)]
pub enum CallTarget<'ast> {
    Function(&'ast Fundef<'ast, TypedAst>),
}

impl<'ast> CallTarget<'ast> {
    pub fn name(&self) -> &str {
        match self {
            CallTarget::Function(f) => &f.name,
        }
    }
}
