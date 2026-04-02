use super::{AstConfig, LocalVar, Expr};

#[derive(Clone, Copy, Debug)]
pub struct Assign<'ast, Ast: AstConfig> {
    pub lvis: &'ast LocalVar<'ast, Ast>,
    pub expr: &'ast Expr<'ast, Ast>,
}
