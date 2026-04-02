use super::*;

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    pub fundefs: Vec<Fundef<'ast, Ast>>,
}
