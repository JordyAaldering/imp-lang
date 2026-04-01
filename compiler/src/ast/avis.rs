use super::AstConfig;

#[derive(Clone, Debug)]
pub struct Avis<Ast: AstConfig> {
    pub name: String,
    pub ty: Ast::ValueType,
}
