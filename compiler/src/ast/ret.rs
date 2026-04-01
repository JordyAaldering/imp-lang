use super::{Id, AstConfig};

#[derive(Clone, Copy, Debug)]
pub struct Return<'ast, Ast: AstConfig> {
    pub id: Id<'ast, Ast>,
}
