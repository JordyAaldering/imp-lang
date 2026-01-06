use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub key: ArgOrVar<Ast>,
    pub ty: Ast::ValueType,
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
