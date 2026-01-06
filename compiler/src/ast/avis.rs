use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub key: ArgOrVar<Ast>,
    pub name: String,
    pub ty: Ast::ValueType,
}

impl<Ast: AstConfig> Avis<Ast> {
    pub fn new(key: ArgOrVar<Ast>, name: &str, ty: Ast::ValueType) -> Self {
        Self { key, name: name.to_owned(), ty }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar<Ast: AstConfig> {
    /// Function argument
    Arg(usize),
    /// Local variable
    Var(Ast::SlotKey),
    /// Index vector
    Iv(Ast::SlotKey),
}
