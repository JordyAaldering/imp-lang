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

    fn pass_id(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Self::SsaOut;

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

/// Same-AST traversal trait with identity defaults.
///
/// Implement this when `InAst == OutAst` and you want to override only a small
/// subset of traversal methods (for example only `pass_u32`).
pub trait AstVisit<'ast> {
    type Ast: AstConfig + 'ast;

    fn pass_program(&mut self, program: Program<'ast, Self::Ast>) -> Program<'ast, Self::Ast> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            fundefs.push(self.pass_fundef(fundef));
        }
        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::Ast>) -> Fundef<'ast, Self::Ast> {
        fundef
    }

    fn pass_expr(&mut self, expr: Expr<'ast, Self::Ast>) -> Expr<'ast, Self::Ast> {
        expr
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::Ast>) -> Tensor<'ast, Self::Ast> {
        tensor
    }

    fn pass_binary(&mut self, binary: Binary<'ast, Self::Ast>) -> Binary<'ast, Self::Ast> {
        binary
    }

    fn pass_unary(&mut self, unary: Unary<'ast, Self::Ast>) -> Unary<'ast, Self::Ast> {
        unary
    }

    fn pass_id(&mut self, id: ArgOrVar<'ast, Self::Ast>) -> ArgOrVar<'ast, Self::Ast> {
        id
    }

    fn pass_bool(&mut self, value: bool) -> bool {
        value
    }

    fn pass_u32(&mut self, value: u32) -> u32 {
        value
    }
}

impl<'ast, T> AstPass<'ast> for T
where
    T: AstVisit<'ast>,
{
    type InAst = T::Ast;

    type OutAst = T::Ast;

    fn pass_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        AstVisit::pass_program(self, program)
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        AstVisit::pass_fundef(self, fundef)
    }

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        AstVisit::pass_expr(self, expr)
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        AstVisit::pass_tensor(self, tensor)
    }

    fn pass_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut {
        AstVisit::pass_binary(self, binary)
    }

    fn pass_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut {
        AstVisit::pass_unary(self, unary)
    }

    fn pass_id(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Self::SsaOut {
        AstVisit::pass_id(self, id)
    }

    fn pass_bool(&mut self, value: bool) -> Self::BoolOut {
        AstVisit::pass_bool(self, value)
    }

    fn pass_u32(&mut self, value: u32) -> Self::U32Out {
        AstVisit::pass_u32(self, value)
    }
}
