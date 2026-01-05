use crate::arena::{Arena, Key, SecondaryArena};

use super::{AstConfig, ArgOrVar, Avis, Expr};

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
///
/// ```
/// fn scope(int ub) -> int[ub] {
///     one = 1;
///     two = 2;
///     ret = { a = { jv + one | jv < ub };
///             b = { jv + two | jv < ub };
///             ub + one + a[iv] + b[iv]    // a and b can be found in the first scope
///                                         // one cannot be found, so go one scope up
///                                         // even there, ub cannot be found, so go up again and check args
///           | iv < ub
///     };
///     return ret;
/// }
/// ```
///
/// This means we might have to look through multiple scopes.
/// The naive approach of passing the fundef to traversal functions in not enough
/// we might have to look through a number of scopes.
#[derive(Clone, Debug)]
pub struct Tensor<Ast: AstConfig> {
    pub iv: Key,
    pub lb: ArgOrVar,
    pub ub: ArgOrVar,
    pub ids: Arena<Avis<Ast>>,
    pub ssa: SecondaryArena<Expr<Ast>>,
    pub ret: ArgOrVar,
}
