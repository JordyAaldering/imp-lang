use crate::arena::Key;

use super::{AstConfig, ArgOrVar, Block};

/// ```
/// { iv + 1 | 0 <= iv < 3;
///   iv - 1 | 3 <= iv < 6;
///   iv + 1 | 6 <= iv < 9; }
/// ```
///
/// Each <expr> (may) contain an index vector variable, which points to the partition/range.
/// Multiple partitions may have the same <expr> (albeit potentially with a different name for
/// the index vector, but it must at least be of the same shape.)
///
/// For now, we assume only a single partition.
#[derive(Clone, Debug)]
pub struct Tensor<Ast: AstConfig> {
    pub body: Block<Ast>,
    pub iv: IndexVector,
    pub lb: ArgOrVar,
    pub ub: ArgOrVar,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexVector(pub Key);
