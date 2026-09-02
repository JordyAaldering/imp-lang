use super::*;

#[derive(Clone, Debug)]
pub struct Array<'ast, Ast: Invariant> {
    pub elems: Vec<Ast::Operand<'ast>>,
}
