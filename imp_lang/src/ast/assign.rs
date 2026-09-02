use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Assign<'ast, Ast: Invariant> {
    pub lhs: &'ast VarInfo<'ast, Ast>,
    pub expr: &'ast ExprCell<'ast, Ast>,
}
