use super::*;

#[derive(Clone, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign(Assign<'ast, Ast>),
    Printf(Printf<'ast, Ast>),
}
