use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Call(Call<'ast, Ast>),
    PrfCall(PrfCall<'ast, Ast>),
    Tensor(Tensor<'ast, Ast>),
    Array(Array<'ast, Ast>),
    // Terminals
    Id(Id<'ast, Ast>),
    // TODO: merge constants into a single Const enum/struct
    Const(Const),
}
