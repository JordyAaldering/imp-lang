use super::*;

#[derive(Clone, Debug)]
pub struct Tensor<'ast, Ast: AstConfig> {
    pub body: Vec<Stmt<'ast, Ast>>,
    pub ret: Ast::Operand<'ast>,
    pub iv: &'ast VarInfo<'ast, Ast>,
    pub lb: Ast::Operand<'ast>,
    pub ub: Ast::Operand<'ast>,
}
