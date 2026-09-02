use super::*;

#[derive(Clone, Debug)]
pub enum Stmt<'ast, Ast: Invariant> {
    Assign(Assign<'ast, Ast>),
    Printf(Printf<'ast, Ast>),
}
