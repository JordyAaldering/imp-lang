use crate::ast::*;

pub trait Rewriter: Scoped<Self::InAst, Self::OutAst> {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Err;

    // If we require setscope and getscope functions, we can automate the scoping and looking up somewhat
    fn trav_program(&mut self, program: Program<Self::InAst>) -> Result<Program<Self::OutAst>, Self::Err> {
        let mut new_program = Program::new();

        for fundef in program.fundefs {
            self.set_fargs(fundef.args.clone());
            self.push_scope(fundef.ids.clone(), fundef.ssa.clone());

            let fundef = self.trav_fundef(fundef)?;
            new_program.fundefs.push(fundef);

            // Potential: here, we can compare the old (cloned) fundef against the updated one
            // which for example allows us to only print changes when debugging
            self.pop_fargs();
            self.pop_scope();
        }

        Ok(new_program)
    }

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err>;

    fn trav_ssa(&mut self, id: ArgOrVar) -> Result<ArgOrVar, Self::Err>;

    fn trav_expr(&mut self, expr: Expr<Self::InAst>) -> Result<Expr<Self::OutAst>, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n).map(Tensor),
            Binary(n) => self.trav_binary(n).map(Binary),
            Unary(n) => self.trav_unary(n).map(Unary),
            Bool(n) => self.trav_bool(n).map(Bool),
            U32(n) => self.trav_u32(n).map(U32),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<Tensor<Self::OutAst>, Self::Err> {
        self.push_scope(tensor.ids.clone(), tensor.ssa.clone());

        let lb = self.trav_ssa(tensor.lb)?;
        let ub = self.trav_ssa(tensor.ub)?;
        let iv = self.trav_iv(tensor.iv)?;
        let ret = self.trav_ssa(tensor.ret)?;

        let (ids, ssa) = self.pop_scope();

        Ok(Tensor { iv, lb, ub, ids, ssa, ret })
    }

    fn trav_iv(&mut self, iv: IndexVector) -> Result<IndexVector, Self::Err>;

    fn trav_binary(&mut self, binary: Binary) -> Result<Binary, Self::Err> {
        let l = self.trav_ssa(binary.l)?;
        let r = self.trav_ssa(binary.r)?;
        Ok(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary) -> Result<Unary, Self::Err> {
        let r = self.trav_ssa(unary.r)?;
        Ok(Unary { r, op: unary.op })
    }

    fn trav_bool(&mut self, value: bool) -> Result<bool, Self::Err> {
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32) -> Result<u32, Self::Err> {
        Ok(value)
    }
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
