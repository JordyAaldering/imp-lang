use std::ops::Deref;

use super::{ArgOrVar, AstConfig};

#[derive(Clone, Debug)]
pub struct Tensor<Ast: AstConfig> {
    pub iv: IndexVector<Ast>,
    pub expr: ArgOrVar<Ast>,
    pub lb: ArgOrVar<Ast>,
    pub ub: ArgOrVar<Ast>,
}

#[derive(Clone, Debug)]
pub struct IndexVector<Ast: AstConfig>(pub Ast::VarKey);

impl<Ast: AstConfig> Deref for IndexVector<Ast> {
    type Target = Ast::VarKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
