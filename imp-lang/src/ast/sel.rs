use super::*;

/// Selection (indexing)
///
/// Example: `arr[i]` or `arr[i,j]` where `i` and `j` are scalars.
///
/// Not yet supported: arr[iv] where `iv` is an index vector.
/// For this, we actually need the scalar cases to be written as `arr[[i]]` and `arr[[i,j]]`.
/// We ignore this for now.
///
/// The built-in selection only allows for scalar selection.
/// I.e. the number of indices must match the number of dimensions of the array,
/// thus returning a scalar value.
#[derive(Clone, Debug)]
pub struct Sel<'ast, Ast: AstConfig> {
    pub arr: Ast::Operand<'ast>,
    pub idx: Vec<Ast::Operand<'ast>>,
}
