use crate::ast::*;

pub trait AstPass<'ast> {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Ok;

    type Err;

    fn pass_program(&mut self, program: Program<'ast, Self::InAst>) -> Result<(Self::Ok, Program<'ast, Self::OutAst>), Self::Err>
    where
        Self::Ok: Default,
    {
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let (_, fundef) = self.pass_fundef(fundef)?;
            fundefs.push(fundef);
        }

        Ok((Self::Ok::default(), Program { fundefs }))
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Result<(Self::Ok, Fundef<'ast, Self::OutAst>), Self::Err>;

    /// Recursively traverse the single static assignment of an identifier
    fn pass_ssa(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Result<(Self::Ok, ArgOrVar<'ast, Self::OutAst>), Self::Err>;

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Result<(Self::Ok, Expr<'ast, Self::OutAst>), Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.pass_tensor(n).map(|(x,n)| (x, Tensor(n))),
            Binary(n) => self.pass_binary(n).map(|(x,n)| (x, Binary(n))),
            Unary(n) => self.pass_unary(n).map(|(x,n)| (x, Unary(n))),
            Bool(n) => self.pass_bool(n).map(|(x,n)| (x, Bool(n))),
            U32(n) => self.pass_u32(n).map(|(x,n)| (x, U32(n))),
        }
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Result<(Self::Ok, Tensor<'ast, Self::OutAst>), Self::Err>;

    fn pass_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Result<(Self::Ok, Binary<'ast, Self::OutAst>), Self::Err>;

    fn pass_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Result<(Self::Ok, Unary<'ast, Self::OutAst>), Self::Err>;

    fn pass_bool(&mut self, value: bool) -> Result<(Self::Ok, bool), Self::Err>;

    fn pass_u32(&mut self, value: u32) -> Result<(Self::Ok, u32), Self::Err>;
}
