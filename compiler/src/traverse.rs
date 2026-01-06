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

/// Maybe it is okay to pass some Scope vector/struct as a readonly reference.
/// Any traversal should only be allowed to modify itself, not its parents.
///
/// Hmmm, nevermind maybe?
/// The problem is that the fundef/scope keeps the ssa definition, which IS
/// in practise the node we are visiting, even though it lives in a parent somewhere...
///
/// Perhaps if we want to do this, we need to force a new entry to be created in the arena?
pub trait Traverse<Ast: AstConfig> {
    type Output;

    const DEFAULT: Self::Output;

    fn trav_program(&mut self, program: &mut Program<Ast>) -> Self::Output {
        for fundef in &mut program.fundefs {
            self.trav_fundef(fundef);
        }
        Self::DEFAULT
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<Ast>) -> Self::Output {
        for arg in &mut fundef.args {
            self.trav_arg(arg);
        }
        self.trav_ssa(&mut fundef.ret);
        Self::DEFAULT
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
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n),
            Binary(n) => self.trav_binary(n),
            Unary(n) => self.trav_unary(n),
            Bool(n) => self.trav_bool(n),
            U32(n) => self.trav_u32(n),
        }
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<Ast>) -> Self::Output {
        self.trav_ssa(&mut tensor.lb);
        self.trav_ssa(&mut tensor.ub);
        self.trav_ssa(&mut tensor.ret);
        Self::DEFAULT
    }

    fn trav_binary(&mut self, binary: &mut Binary<Ast>) -> Self::Output {
        self.trav_ssa(&mut binary.r);
        self.trav_ssa(&mut binary.r);
        Self::DEFAULT
    }

    fn trav_unary(&mut self, unary: &mut Unary<Ast>) -> Self::Output {
        self.trav_ssa(&mut unary.r);
        Self::DEFAULT
    }

    fn trav_bool(&mut self, _: &mut bool) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_u32(&mut self, _: &mut u32) -> Self::Output {
        Self::DEFAULT
    }
}
