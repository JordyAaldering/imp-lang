use super::*;

#[derive(Clone, Debug)]
pub struct Body<'ast, Ast: Invariant> {
    pub stmts: Vec<Stmt<'ast, Ast>>,
    pub ret: Ast::Operand<'ast>,
}
