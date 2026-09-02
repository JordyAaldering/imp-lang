use super::*;

#[derive(Clone, Debug)]
pub struct Array<'ast, Ast: AstConfig> {
    pub elems: Vec<Ast::Operand<'ast>>,
}
