use crate::ast::*;

pub trait Visit<Ast, W>
    where
        Ast: AstConfig,
        W: Walk<Ast>,
{
    fn visit(&self, walk: &mut W) -> W::Output;
}

pub trait Walk<Ast>
where
    Self: Sized,
    Ast: AstConfig,
{
    type Output;

    const DEFAULT: Self::Output;

    fn trav_program(&mut self, program: &Program<Ast>) -> Self::Output {
        program.visit(self)
    }

    fn trav_fundef(&mut self, fundef: &Fundef<Ast>) -> Self::Output {
        fundef.visit(self)
    }

    fn trav_arg(&mut self, _arg: &Avis<Ast>) -> Self::Output {
        Self::DEFAULT
    }

    /// An identifier was encountered in an expression position.
    ///
    /// Recursively traverse the single static assignment of the identifier.
    fn trav_ssa(&mut self, _id: &ArgOrVar<Ast>) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_expr(&mut self, expr: &Expr<Ast>) -> Self::Output {
        expr.visit(self)
    }

    fn trav_tensor(&mut self, tensor: &Tensor<Ast>) -> Self::Output {
        tensor.visit(self)
    }

    fn trav_binary(&mut self, binary: &Binary<Ast>) -> Self::Output {
        binary.visit(self)
    }

    fn trav_unary(&mut self, unary: &Unary<Ast>) -> Self::Output {
        unary.visit(self)
    }

    fn trav_bool(&mut self, _: &bool) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_u32(&mut self, _: &u32) -> Self::Output {
        Self::DEFAULT
    }
}
