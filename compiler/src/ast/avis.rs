use slotmap::DefaultKey;

use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub key: ArgOrVar,
    pub name: String,
    pub ty: Ast::ValueType,
}

impl<Ast: AstConfig> Avis<Ast> {
    pub fn new(key: ArgOrVar, name: &str, ty: Ast::ValueType) -> Self {
        Self { key, name: name.to_owned(), ty }
    }

    pub fn from<T: AstConfig>(avis: &Avis<T>, ty: Ast::ValueType) -> Self {
        Self { key: avis.key, name: avis.name.to_owned(), ty }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(DefaultKey),
    /// Index vector
    Iv(DefaultKey),
}
