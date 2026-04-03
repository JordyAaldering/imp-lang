use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Call(Call<'ast, Ast>),
    Tensor(Tensor<'ast, Ast>),
    Binary(Binary<'ast, Ast>),
    Unary(Unary<'ast, Ast>),
    Array(Array<'ast, Ast>),
    Sel(Sel<'ast, Ast>),
    // Terminals
    Id(Id<'ast, Ast>),
    Bool(bool),
    U32(u32),
}
