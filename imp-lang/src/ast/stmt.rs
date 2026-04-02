use super::*;

#[derive(Clone, Debug)]
pub enum Stmt<'ast, Ast: AstConfig> {
    Assign(Assign<'ast, Ast>),
    Return(Return<'ast, Ast>),
}
