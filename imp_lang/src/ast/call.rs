use super::*;

#[derive(Clone, Debug)]
pub struct Call<'ast, Ast: Invariant> {
    pub id: Ast::Dispatch<'ast>,
    pub args: Vec<Ast::Operand<'ast>>,
}
