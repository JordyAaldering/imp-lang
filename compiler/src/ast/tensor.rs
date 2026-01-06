use slotmap::SecondaryMap;

use crate::visit::{Visit, Walk};

use super::{AstConfig, ArgOrVar, Expr};

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
    pub ssa: SecondaryMap<Ast::SlotKey, Expr<Ast>>,
    pub iv: Ast::SlotKey,
    pub lb: ArgOrVar<Ast>,
    pub ub: ArgOrVar<Ast>,
    pub ret: ArgOrVar<Ast>,
}

impl<Ast, W> Visit<Ast, W> for Tensor<Ast>
where
    Ast: AstConfig,
    W: Walk<Ast>,
{
    fn visit(&self, walk: &mut W) -> W::Output {
        walk.trav_ssa(&self.lb);
        walk.trav_ssa(&self.ub);
        walk.trav_ssa(&self.ret);
        W::DEFAULT
    }
}
