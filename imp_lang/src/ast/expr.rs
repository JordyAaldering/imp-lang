use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Cond(Cond<'ast, Ast>),
    Call(Call<'ast, Ast>),
    Prf(Prf<'ast, Ast>),
    Tensor(Tensor<'ast, Ast>),
    Fold(Fold<'ast, Ast>),
    Array(Array<'ast, Ast>),
    Id(Id<'ast, Ast>),
    Const(Const),
}
