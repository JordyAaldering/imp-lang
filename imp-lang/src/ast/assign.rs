use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Assign<'ast, Ast: AstConfig> {
    pub lhs: &'ast VarInfo<'ast, Ast>,
    pub expr: &'ast Expr<'ast, Ast>,
}
