use super::{Id, AstConfig, Avis, ScopeBlock, ScopeEntry, Stmt};

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
///     a = whatever;
///     ret = { a = { jv + one | jv < ub }; // This should shadow `a` from the outer scope
///             b = { jv + two | jv < ub }; // This should match the local jv, not the one from the previous scope
///             ub + one + a[iv] + b[iv]    // a and b can be found in the first scope
///                                         // one cannot be found, so go one scope up
///                                         // even there, ub cannot be found, so go up again and check args
///           | 0 <= iv < ub };
///     return ret;
/// }
/// ```
///
/// This means we might have to look through multiple scopes.
/// The naive approach of passing the fundef to traversal functions in not enough
/// we might have to look through a number of scopes.
#[derive(Clone, Debug)]
pub struct Tensor<'ast, Ast: AstConfig> {
    pub body: Vec<Stmt<'ast, Ast>>,
    pub ret: Id<'ast, Ast>,
    pub iv: &'ast Avis<Ast>,
    pub lb: Id<'ast, Ast>,
    pub ub: Id<'ast, Ast>,
}

impl<'ast, Ast: AstConfig> Tensor<'ast, Ast> {
    pub fn scope_block(&self) -> ScopeBlock<'ast, Ast> {
        let mut scope = Vec::with_capacity(1 + self.body.len());
        scope.push(ScopeEntry::IndexRange {
            iv: self.iv,
            lb: self.lb,
            ub: self.ub,
        });

        for stmt in &self.body {
            if let Some(entry) = (*stmt).as_scope_entry() {
                scope.push(entry);
            }
        }

        scope
    }
}
