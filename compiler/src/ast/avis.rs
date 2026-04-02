use super::AstConfig;

/// Argument or Variable Information Structure
#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::VarType,
}
