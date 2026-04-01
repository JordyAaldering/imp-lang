use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
}

#[derive(Clone, Copy, Debug)]
pub enum ArgOrVar<'ast, Ast: AstConfig> {
    /// Function argument
    Arg(usize),
    /// Local variable, including tensor index variables.
    Var(&'ast Avis<Ast>),
}

impl<'ast, Ast: AstConfig> ArgOrVar<'ast, Ast> {
    pub fn as_local(self) -> Option<&'ast Avis<Ast>> {
        match self {
            Self::Arg(_) => None,
            Self::Var(avis) => Some(avis),
        }
    }
}
