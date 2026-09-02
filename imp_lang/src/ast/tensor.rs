use super::*;

/// ```bnf
/// { <stmt>* <expr> | <lb> <= <iv> < <ub> }
/// ```
///
/// Where <lb> and <ub> must be vectors of the same shape.
/// <iv> is a variable that iterates over the range [<lb>, <ub>).
/// Where <stmt> and <expr> can refer to the induction variable <iv>.
/// The shape of the result is shape(<expr>) ++ <ub>.
/// Note that <lb> has no effect on the result shape. Any values below <lb> are zeros.
#[derive(Clone, Debug)]
pub struct Tensor<'ast, Ast: AstConfig> {
    pub body: Body<'ast, Ast>,
    pub iv: &'ast VarInfo<'ast, Ast>,
    pub lb: Option<Ast::Operand<'ast>>,
    pub ub: Ast::Operand<'ast>,
}
