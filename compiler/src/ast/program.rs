use super::{AstConfig, Fundef};

#[derive(Clone, Debug)]
pub struct Program<Ast: AstConfig> {
    pub fundefs: Vec<Fundef<Ast>>,
}

impl<Ast: AstConfig> Program<Ast> {
    pub fn new() -> Self {
        Self {
            fundefs: Vec::new(),
        }
    }
}
