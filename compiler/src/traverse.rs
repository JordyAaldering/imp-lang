use crate::ast::*;

/// AST traversal and transformation pass trait.
///
/// Implementations define how to walk and potentially transform an AST.
/// Each pass has an input AST type (InAst) and output AST type (OutAst).
///
/// Associated type defaults allow flexible expression output transformations:
/// - If a pass doesn't transform a node type, the default is an identity output
/// - Passes can override specific output types to enable rewrites
///   (e.g., constant folding Binary -> U32)
pub trait AstPass<'ast> {
    type InAst: AstConfig;

    type OutAst: AstConfig + 'ast;

    fn pass_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.pass_fundef(fundef);
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst>;

    type SsaOut = ArgOrVar<'ast, Self::OutAst>;

    fn pass_ssa(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Self::SsaOut;

    type ExprOut = Expr<'ast, Self::OutAst>;

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut;

    type TensorOut = Tensor<'ast, Self::OutAst>;

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut;

    type BinaryOut = Binary<'ast, Self::OutAst>;

    fn pass_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut;

    type UnaryOut = Unary<'ast, Self::OutAst>;

    fn pass_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut;

    type BoolOut = bool;

    fn pass_bool(&mut self, value: bool) -> Self::BoolOut;

    type U32Out = u32;

    fn pass_u32(&mut self, value: u32) -> Self::U32Out;
}
