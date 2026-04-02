use super::*;

#[derive(Clone, Debug)]
pub struct Array<'ast, Ast: AstConfig> {
    pub values: Vec<Ast::Operand<'ast>>,
}
