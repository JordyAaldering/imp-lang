use super::AstConfig;

/// Formal argument
#[derive(Clone, Debug)]
pub struct Farg<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
}

/// Local SSA variable
#[derive(Clone, Debug)]
pub struct Lvis<'ast, Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
    pub ssa: Ast::SsaLink<'ast>,
}
