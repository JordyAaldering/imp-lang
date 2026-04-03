use super::*;

/// Selection (indexing)
///
/// Example: `arr[i]` or `arr[i,j]` where `i` and `j` are scalars.
/// Or arr[iv] where `iv` is a scalar, or an index vector.
///
/// The built-in selection only allows for scalar selection.
/// I.e. the number of indices must match the number of dimensions of the array,
/// thus returning a scalar value.
#[derive(Clone, Debug)]
pub struct Sel<'ast, Ast: AstConfig> {
    pub arr: Ast::Operand<'ast>,
    pub idx: Vec<Ast::Operand<'ast>>,
}
