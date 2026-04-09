use super::*;

#[derive(Clone, Debug)]
pub struct Cond<'ast, Ast: AstConfig> {
    pub cond: Ast::Operand<'ast>,
    pub true_branch: Ast::Operand<'ast>,
    pub false_branch: Ast::Operand<'ast>,
}
