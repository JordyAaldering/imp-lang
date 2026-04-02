use super::AstConfig;

/// Local SSA variable
#[derive(Clone, Debug)]
pub struct Lvis<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}
