use super::{AstConfig, Lvis, Expr};

#[derive(Clone, Copy, Debug)]
pub struct Assign<'ast, Ast: AstConfig> {
    pub lvis: &'ast Lvis<'ast, Ast>,
    pub expr: &'ast Expr<'ast, Ast>,
}
