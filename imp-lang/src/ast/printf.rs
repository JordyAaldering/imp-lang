use super::*;

#[derive(Clone, Debug)]
pub struct Printf<'ast, Ast: AstConfig> {
    pub id: Id<'ast, Ast>,
}
