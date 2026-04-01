use crate::ast::*;

pub trait Rewriter<'ast> {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Ok;

    type Err;

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Result<(Self::Ok, Fundef<'ast, Self::OutAst>), Self::Err>;

    /// Recursively traverse the single static assignment of an identifier
    fn trav_ssa(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Result<(Self::Ok, ArgOrVar<'ast, Self::OutAst>), Self::Err>;

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Result<(Self::Ok, Expr<'ast, Self::OutAst>), Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n).map(|(x,n)| (x, Tensor(n))),
            Binary(n) => self.trav_binary(n).map(|(x,n)| (x, Binary(n))),
            Unary(n) => self.trav_unary(n).map(|(x,n)| (x, Unary(n))),
            Bool(n) => self.trav_bool(n).map(|(x,n)| (x, Bool(n))),
            U32(n) => self.trav_u32(n).map(|(x,n)| (x, U32(n))),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Result<(Self::Ok, Tensor<'ast, Self::OutAst>), Self::Err>;

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Result<(Self::Ok, Binary<'ast, Self::OutAst>), Self::Err>;

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Result<(Self::Ok, Unary<'ast, Self::OutAst>), Self::Err>;

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
///
/// That's a huge pain. Perhaps all nodes but the fundef should be trivially easy,
/// and then the fundef contains ALL scopes.
/// Unsure how to then use this to figure out the ordering of the scopes though
/// Perhaps just some sort of tree structure.
/// Then everything that has a scope (such as the tensor comprehension) also has its own unique key,
/// which we can use to determine which tree path to pick?
/// And then whilst traversing the AST we move down into the tree when necessary?
/// (But what is we have another key _within_ this tensor comprehension? For that one we do not know the scope...
/// Maybe we need to keep track of the current scope index in the knapsack?)
pub trait Traverse<'ast, Ast: AstConfig> {
    type Output;

    const DEFAULT: Self::Output;

    fn trav_program(&mut self, program: &mut Program<'ast, Ast>) -> Self::Output {
        for fundef in &mut program.fundefs {
            self.trav_fundef(fundef);
        }
        Self::DEFAULT
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Ast>) -> Self::Output {
        for arg in &mut fundef.args {
            self.trav_arg(arg);
        }
        self.trav_ssa(&mut fundef.ret);
        Self::DEFAULT
    }

    fn trav_arg(&mut self, _arg: &mut &'ast Avis<'ast, Ast>) -> Self::Output {
        Self::DEFAULT
    }

    /// An identifier was encountered in an expression position.
    ///
    /// Recursively traverse the single static assignment of the identifier.
    fn trav_ssa(&mut self, _id: &mut ArgOrVar<'ast, Ast>) -> Self::Output {
        Self::DEFAULT
    }

    fn trav_expr(&mut self, expr: &mut Expr<'ast, Ast>) -> Self::Output {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n),
            Binary(n) => self.trav_binary(n),
            Unary(n) => self.trav_unary(n),
            Bool(n) => self.trav_bool(n),
            U32(n) => self.trav_u32(n),
        }
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, Ast>) -> Self::Output {
        self.trav_ssa(&mut tensor.lb);
        self.trav_ssa(&mut tensor.ub);
        self.trav_ssa(&mut tensor.ret);
        Self::DEFAULT
    }

    fn trav_binary(&mut self, binary: &mut Binary<'ast, Ast>) -> Self::Output {
        self.trav_ssa(&mut binary.r);
        self.trav_ssa(&mut binary.r);
        Self::DEFAULT
    }

    fn trav_unary(&mut self, unary: &mut Unary<'ast, Ast>) -> Self::Output {
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
