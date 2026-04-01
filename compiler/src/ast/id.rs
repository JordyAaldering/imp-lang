use super::{AstConfig, Avis};

#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: AstConfig> {
    Arg(usize),
    Var(&'ast Avis<Ast>),
}

impl<'ast, Ast: AstConfig> Id<'ast, Ast> {
    pub fn as_local(self) -> Option<&'ast Avis<Ast>> {
        match self {
            Self::Arg(_) => None,
            Self::Var(avis) => Some(avis),
        }
    }
}
