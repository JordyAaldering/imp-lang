use super::*;

#[derive(Clone, Debug)]
pub enum Expr<'ast, Ast: AstConfig> {
    Call(Call<'ast, Ast>),
    PrfCall(PrfCall<'ast, Ast>),
    Tensor(Tensor<'ast, Ast>),
    Array(Array<'ast, Ast>),
    // Terminals
    Id(Id<'ast, Ast>),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    Usize(usize),
    F32(f32),
    F64(f64),
    Bool(bool),
}
