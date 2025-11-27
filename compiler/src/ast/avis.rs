use crate::arena::Key;

use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
    pub key: ArgOrVar,
}

impl<Ast: AstConfig> Avis<Ast> {
    pub fn new(key: ArgOrVar, name: &str, ty: Ast::ValueType) -> Self {
        Self { name: name.to_owned(), ty, key }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(Key),
    /// Index vector
    Iv(Key),
}
