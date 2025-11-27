use crate::arena::Key;

use super::ArgOrVar;

#[derive(Clone, Debug)]
pub struct Tensor {
    pub iv: IndexVector,
    pub expr: ArgOrVar,
    pub lb: ArgOrVar,
    pub ub: ArgOrVar,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexVector(pub Key);
