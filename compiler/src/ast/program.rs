use super::{AstConfig, Fundef};

#[derive(Clone, Debug)]
pub struct Program<'ast, Ast: AstConfig> {
    pub fundefs: Vec<Fundef<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> Program<'ast, Ast> {
    pub fn new() -> Self {
        Self {
            fundefs: Vec::new(),
        }
    }
}
