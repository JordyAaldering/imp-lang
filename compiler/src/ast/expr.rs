use crate::visit::{Visit, Walk};

use super::{AstConfig, Tensor, Binary, Unary};

#[derive(Clone, Debug)]
pub enum Expr<Ast: AstConfig> {
    Tensor(Tensor<Ast>),
    Binary(Binary<Ast>),
    Unary(Unary<Ast>),
    // We might be able to drop constants, if we include this in the type information instead
    // - maybe not actually, as a varkey should come with an ssa as well. But maybe a type in the field of Const does work
    // - or alternatively, a map of constants alongside the ssa map
    Bool(bool),
    U32(u32),
}

impl<Ast, W> Visit<Ast, W> for Expr<Ast>
where
    Ast: AstConfig,
    W: Walk<Ast>,
{
    fn visit(&mut self, walk: &mut W) -> W::Output {
        use Expr::*;
        match self {
            Tensor(n) => walk.trav_tensor(n),
            Binary(n) => walk.trav_binary(n),
            Unary(n) => walk.trav_unary(n),
            Bool(n) => walk.trav_bool(n),
            U32(n) => walk.trav_u32(n),
        }
    }
}
