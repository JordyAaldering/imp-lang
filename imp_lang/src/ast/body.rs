use super::*;

#[derive(Clone, Debug)]
pub struct Body<'ast, Ast: AstConfig> {
    pub stmts: Vec<Stmt<'ast, Ast>>,
    pub ret: Ast::Operand<'ast>,
}
