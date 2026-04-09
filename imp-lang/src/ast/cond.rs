use super::*;

#[derive(Clone, Debug)]
pub struct Cond<'ast, Ast: AstConfig> {
    pub cond: Ast::Operand<'ast>,
    pub then_branch: Ast::Operand<'ast>,
    pub else_branch: Ast::Operand<'ast>,
}
