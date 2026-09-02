use super::*;

#[derive(Clone, Debug)]
pub struct Printf<'ast, Ast: Invariant> {
    pub id: Id<'ast, Ast>,
}
