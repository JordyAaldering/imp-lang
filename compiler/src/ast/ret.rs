use super::{ArgOrVar, AstConfig};

#[derive(Clone, Copy, Debug)]
pub struct Return<'ast, Ast: AstConfig> {
    pub id: ArgOrVar<'ast, Ast>,
}
