use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Tensor(Tensor<'ast, Ast>),
    Binary(Binary<'ast, Ast>),
    Unary(Unary<'ast, Ast>),
    Array(Array<'ast, Ast>),
    // Terminals
    Id(Id<'ast, Ast>),
    Bool(bool),
    U32(u32),
}
