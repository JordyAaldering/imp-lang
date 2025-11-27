use super::{AstConfig, Tensor, Binary, Unary};

#[derive(Clone, Debug)]
pub enum Expr<Ast: AstConfig> {
    Tensor(Tensor<Ast>),
    Binary(Binary<Ast>),
    Unary(Unary<Ast>),
    // I don't think var is actually needed. During parsing we do still need such a construct because we lack context
    // (A slotmap does not even exist yet, everything is just identifiers that may or may not exist)
    // But afterwards it is redundant
    //Var(VarKey),
    // We might even be able to do the same thing for constants, if we include this in the type information instead
    // - maybe not actually, as a varkey should come with an ssa as well. But maybe a type in the field of Const does work
    // - or alternatively, a map of constants alongside the ssa map
    Bool(bool),
    U32(u32),
}
