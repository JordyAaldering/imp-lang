use super::{Id, AstConfig};

#[derive(Clone, Debug)]
pub struct Return<'ast, Ast: AstConfig> {
    pub id: Id<'ast, Ast>,
}
