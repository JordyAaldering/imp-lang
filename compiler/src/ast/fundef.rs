use std::ops;

use crate::ast::Block;

use super::{ArgOrVar, AstConfig, Avis};

#[derive(Clone, Debug)]
pub struct Fundef<Ast: AstConfig> {
    pub name: String,
    pub args: Vec<Avis<Ast>>,
    pub body: Block<Ast>,
}

impl<Ast: AstConfig> ops::Index<ArgOrVar> for Fundef<Ast> {
    type Output = Avis<Ast>;

    fn index(&self, x: ArgOrVar) -> &Self::Output {
        match x {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.body.ids[k],
            ArgOrVar::Iv(k) => &self.body.ids[k],
        }
    }
}
