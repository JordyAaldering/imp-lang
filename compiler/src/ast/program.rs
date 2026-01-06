use crate::visit::{Visit, Walk};

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

impl<Ast, W> Visit<Ast, W> for Program<Ast>
where
    Ast: AstConfig,
    W: Walk<Ast>,
{
    fn visit(&self, walk: &mut W) -> W::Output {
        for fundef in &self.fundefs {
            walk.trav_fundef(fundef);
        }
        W::DEFAULT
    }
}
