use super::*;

/// Function application (call)
///
/// Example: `arr[iv]` where `iv` is a rank-1 integer vector.
///
/// Scalar selections are written by constructing an explicit index vector,
/// e.g. `arr[[i]]` or `arr[[i,j]]`.
#[derive(Clone, Debug)]
pub struct Call<'ast, Ast: AstConfig> {
    pub id: Ast::Dispatch<'ast>,
    pub args: Vec<Ast::Operand<'ast>>,
}
