use super::*;

/// Function application (call)
///
/// Example: `foo(x, y)` where `x` and `y` are operands (Ids or expressions).
///
/// The `id` field holds the function's dispatch (name or identifier),
/// and `args` holds the argument operands.
#[derive(Clone, Debug)]
pub struct Call<'ast, Ast: AstConfig> {
    pub id: Ast::Dispatch<'ast>,
    pub args: Vec<Ast::Operand<'ast>>,
}
