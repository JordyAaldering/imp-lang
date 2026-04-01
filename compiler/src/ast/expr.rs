use super::{AstConfig, Tensor, Binary, Unary};

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Tensor(Tensor<'ast, Ast>),
    Binary(Binary<'ast, Ast>),
    Unary(Unary<'ast, Ast>),
    // We might be able to drop constants, if we include this in the type information instead
    // - maybe not actually, as a varkey should come with an ssa as well. But maybe a type in the field of Const does work
    // - or alternatively, a map of constants alongside the ssa map
    Bool(bool),
    U32(u32),
}
