use super::{AstConfig, Avis, Expr};

#[derive(Clone, Copy, Debug)]
pub struct Assign<'ast, Ast: AstConfig> {
    pub avis: &'ast Avis<Ast>,
    pub expr: &'ast Expr<'ast, Ast>,
}
