use crate::ast::*;

pub trait Rewriter: Scoped<Self::InAst, Self::OutAst> {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Ok;

    type Err;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<(Self::Ok, Fundef<Self::OutAst>), Self::Err>;

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

    /// Recursively traverse the single static assignment of an identifier
    fn trav_ssa(&mut self, id: ArgOrVar) -> Result<(Self::Ok, ArgOrVar), Self::Err>;

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<(Self::Ok, Tensor<Self::OutAst>), Self::Err>;

    fn trav_binary(&mut self, binary: Binary) -> Result<(Self::Ok, Binary), Self::Err>;

    fn trav_unary(&mut self, unary: Unary) -> Result<(Self::Ok, Unary), Self::Err>;

    fn trav_bool(&mut self, value: bool) -> Result<(Self::Ok, bool), Self::Err>;

    fn trav_u32(&mut self, value: u32) -> Result<(Self::Ok, u32), Self::Err>;
}

pub trait Traversal<Ast: AstConfig>: Scoped<Ast> {
    type Ok;

    type Err;

    const DEFAULT: Result<Self::Ok, Self::Err>;

    fn trav_program(&mut self, program: &mut Program<Ast>) -> Result<Self::Ok, Self::Err> {
        for fundef in &mut program.fundefs {
            self.trav_fundef(fundef)?;
        }
        Self::DEFAULT
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<Ast>) -> Result<Self::Ok, Self::Err> {
        self.set_fargs(fundef.args.clone());
        self.push_scope(fundef.ids.clone(), fundef.ssa.clone());

        for arg in &mut fundef.args {
            self.trav_farg(arg)?;
        }
        self.trav_ssa(&mut fundef.ret)?;

        // Potential: here, we can compare the old (cloned) fundef against the updated one
        // which for example allows us to only print changes when debugging
        self.pop_fargs();
        self.pop_scope();

        Self::DEFAULT
    }

    fn trav_farg(&mut self, _: &mut Avis<Ast>) -> Result<Self::Ok, Self::Err> {
        Self::DEFAULT
    }

    fn trav_ssa(&mut self, _: &mut ArgOrVar) -> Result<Self::Ok, Self::Err> {
        Self::DEFAULT
    }

    fn trav_expr(&mut self, expr: &mut Expr<Ast>) -> Result<Self::Ok, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n)?,
            Binary(n) => self.trav_binary(n)?,
            Unary(n) => self.trav_unary(n)?,
            Bool(n) => self.trav_bool(n)?,
            U32(n) => self.trav_u32(n)?,
        };
        Self::DEFAULT
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<Ast>) -> Result<Self::Ok, Self::Err> {
        self.push_scope(tensor.ids.clone(), tensor.ssa.clone());

        self.trav_ssa(&mut tensor.lb)?;
        self.trav_ssa(&mut tensor.ub)?;
        self.trav_ssa(&mut tensor.ret)?;

        self.pop_scope();

        Self::DEFAULT
    }

    fn trav_binary(&mut self, binary: &mut Binary) -> Result<Self::Ok, Self::Err> {
        self.trav_ssa(&mut binary.l)?;
        self.trav_ssa(&mut binary.r)?;
        Self::DEFAULT
    }

    fn trav_unary(&mut self, unary: &mut Unary) -> Result<Self::Ok, Self::Err> {
        self.trav_ssa(&mut unary.r)?;
        Self::DEFAULT
    }

    fn trav_bool(&mut self, _: &mut bool) -> Result<Self::Ok, Self::Err> {
        Self::DEFAULT
    }

    fn trav_u32(&mut self, _: &mut u32) -> Result<Self::Ok, Self::Err> {
        Self::DEFAULT
    }
}
