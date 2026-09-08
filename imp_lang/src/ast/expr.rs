use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: Invariant> {
    Cond(Cond<'ast, Ast>),
    Call(Call<'ast, Ast>),
    Prf(Prf<'ast, Ast>),
    Tensor(Tensor<'ast, Ast>),
    Fold(Fold<'ast, Ast>),
    Array(Array<'ast, Ast>),
    Id(Id<'ast, Ast>),
    Const(Const),
}

impl<'ast, Ast: Invariant> Expr<'ast, Ast> {
    pub fn visit<Trav>(self, trav: &mut Trav) -> (Self, Trav::ExprOut)
    where
        Trav: Traverse<'ast, Ast = Ast>,
    {
        match self {
            Self::Cond(n) => trav.trav_cond_expr(n),
            Self::Call(n) => trav.trav_call_expr(n),
            Self::Prf(n) => trav.trav_prf_expr(n),
            Self::Tensor(n) => trav.trav_tensor_expr(n),
            Self::Fold(n) => trav.trav_fold_expr(n),
            Self::Array(n) => trav.trav_array_expr(n),
            Self::Id(n) => trav.trav_id_expr(n),
            Self::Const(n) => trav.trav_const_expr(n),
        }
    }
}
