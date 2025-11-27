use super::{ArgOrVar, AstConfig};

#[derive(Clone, Debug)]
pub struct Tensor<Ast: AstConfig> {
    pub expr: ArgOrVar<Ast>,
    pub iv: String,
    pub lb: usize,
    pub ub: usize,
}
