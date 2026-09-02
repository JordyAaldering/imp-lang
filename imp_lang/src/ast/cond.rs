use super::*;

#[derive(Clone, Debug)]
pub struct Cond<'ast, Ast: Invariant> {
    pub cond: Ast::Operand<'ast>,
    pub then_branch: Body<'ast, Ast>,
    pub else_branch: Body<'ast, Ast>,
}
