use super::AstConfig;

#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: AstConfig> {
    Arg(usize),
    Var(Ast::VarLink<'ast>),
}

#[derive(Clone, Debug)]
pub struct LocalVar<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}

impl<'ast, Ast: AstConfig> Id<'ast, Ast> {
    pub fn as_local(&self) -> Option<&Ast::VarLink<'ast>> {
        match self {
            Self::Arg(_) => None,
            Self::Var(link) => Some(link),
        }
    }
}
