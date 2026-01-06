use crate::ast::*;

pub trait Visit<Ast, W>
    where
        Ast: AstConfig,
        W: Walk<Ast>,
{
    fn visit(&mut self, walk: &mut W) -> W::Output;
}

pub trait Walk<Ast>
where
    Self: Sized,
    Ast: AstConfig,
{
    type Output;

    const DEFAULT: Self::Output;

    fn trav_program(&mut self, program: &mut Program<Ast>) -> Self::Output {
        program.visit(self)
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<Ast>) -> Self::Output {
        fundef.visit(self)
    }

    fn trav_arg(&mut self, _arg: &mut Avis<Ast>) -> Self::Output {
        Self::DEFAULT
    }

    /// An identifier was encountered in an expression position.
    ///
    /// Recursively traverse the single static assignment of the identifier.
    fn trav_ssa(&mut self, _id: &mut ArgOrVar<Ast>) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_expr(&mut self, expr: &mut Expr<Ast>) -> Self::Output {
        expr.visit(self)
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<Ast>) -> Self::Output {
        tensor.visit(self)
    }

    fn trav_binary(&mut self, binary: &mut Binary<Ast>) -> Self::Output {
        binary.visit(self)
    }

    fn trav_unary(&mut self, unary: &mut Unary<Ast>) -> Self::Output {
        unary.visit(self)
    }

    fn trav_bool(&mut self, _: &mut bool) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_u32(&mut self, _: &mut u32) -> Self::Output {
        Self::DEFAULT
    }
}
