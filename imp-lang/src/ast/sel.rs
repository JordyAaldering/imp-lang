use super::*;

/// Selection (indexing)
///
/// Example: `arr[iv]` where `iv` is a rank-1 integer vector.
///
/// Scalar selections are written by constructing an explicit index vector,
/// e.g. `arr[[i]]` or `arr[[i,j]]`.
#[derive(Clone, Debug)]
pub struct Sel<'ast, Ast: AstConfig> {
    pub arr: Ast::Operand<'ast>,
    pub idx: Ast::Operand<'ast>,
}
