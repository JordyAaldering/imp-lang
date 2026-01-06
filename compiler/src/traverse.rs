use crate::ast::*;

pub trait Rewriter {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Ok;

    type Err;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<(Self::Ok, Fundef<Self::OutAst>), Self::Err>;

    /// Recursively traverse the single static assignment of an identifier
    fn trav_ssa(&mut self, id: ArgOrVar<Self::InAst>) -> Result<(Self::Ok, ArgOrVar<Self::OutAst>), Self::Err>;

    fn trav_expr(&mut self, expr: Expr<Self::InAst>) -> Result<(Self::Ok, Expr<Self::OutAst>), Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n).map(|(x,n)| (x, Tensor(n))),
            Binary(n) => self.trav_binary(n).map(|(x,n)| (x, Binary(n))),
            Unary(n) => self.trav_unary(n).map(|(x,n)| (x, Unary(n))),
            Bool(n) => self.trav_bool(n).map(|(x,n)| (x, Bool(n))),
            U32(n) => self.trav_u32(n).map(|(x,n)| (x, U32(n))),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<(Self::Ok, Tensor<Self::OutAst>), Self::Err>;

    fn trav_binary(&mut self, binary: Binary<Self::InAst>) -> Result<(Self::Ok, Binary<Self::OutAst>), Self::Err>;

    fn trav_unary(&mut self, unary: Unary<Self::InAst>) -> Result<(Self::Ok, Unary<Self::OutAst>), Self::Err>;

    fn trav_bool(&mut self, value: bool) -> Result<(Self::Ok, bool), Self::Err>;

    fn trav_u32(&mut self, value: u32) -> Result<(Self::Ok, u32), Self::Err>;
}
