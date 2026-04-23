use super::*;

/// Identifier occurring in an expression position.
#[derive(Clone, Copy, Debug)]
pub enum Id<'ast, Ast: AstConfig> {
    /// Formal function argument
    Arg(usize),
    /// Local variable
    Var(Ast::VarLink<'ast>),
}

#[derive(Clone, Debug)]
pub struct VarInfo<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}
