use crate::ast::*;

pub trait AstPass<'ast> {
    type InAst: AstConfig;

    type OutAst: AstConfig + 'ast;

    type ExprOk;

    type SsaOut = ArgOrVar<'ast, Self::OutAst>;
    type ExprOut = Expr<'ast, Self::OutAst>;
    type TensorOut = Tensor<'ast, Self::OutAst>;
    type BinaryOut = Binary<'ast, Self::OutAst>;
    type UnaryOut = Unary<'ast, Self::OutAst>;
    type BoolOut = bool;
    type U32Out = u32;

    fn pass_program(&mut self, program: Program<'ast, Self::InAst>) -> Program<'ast, Self::OutAst> {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.pass_fundef(fundef);
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst>;

    /// Recursively traverse the single static assignment of an identifier
    fn pass_ssa(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> (Self::ExprOk, Self::SsaOut);

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> (Self::ExprOk, Self::ExprOut);

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> (Self::ExprOk, Self::TensorOut);

    fn pass_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> (Self::ExprOk, Self::BinaryOut);

    fn pass_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> (Self::ExprOk, Self::UnaryOut);

    fn pass_bool(&mut self, value: bool) -> (Self::ExprOk, Self::BoolOut);

    fn pass_u32(&mut self, value: u32) -> (Self::ExprOk, Self::U32Out);
}
