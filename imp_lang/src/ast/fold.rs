use super::*;

#[derive(Clone, Debug)]
pub struct Fold<'ast, Ast: AstConfig> {
    pub neutral: Ast::Operand<'ast>,
    pub foldfun: FoldFun<'ast, Ast>,
    pub selection: Tensor<'ast, Ast>,
}

#[derive(Clone, Debug)]
pub enum FoldFun<'ast, Ast: AstConfig> {
    // Implicit binary form: f(acc, elem)
    Name(Ast::Dispatch<'ast>),
    // Placeholder form: f(a, _, b, _, c)
    Apply {
        id: Ast::Dispatch<'ast>,
        args: Vec<FoldFunArg<'ast, Ast>>,
    },
}

#[derive(Clone, Debug)]
pub enum FoldFunArg<'ast, Ast: AstConfig> {
    Placeholder,
    Bound(Ast::Operand<'ast>),
}
