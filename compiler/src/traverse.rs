use crate::ast::*;

pub trait Visit<In> {
    type Out;

    fn visit(&mut self, node: In) -> Self::Out;
}

pub trait Rewriter {
    type InAst: AstConfig;

    type OutAst: AstConfig;

    type Err;

    fn trav_program(&mut self, program: Program<Self::InAst>) -> Result<Program<Self::OutAst>, Self::Err> {
        let mut new_program = Program::new();

        for fundef in program.fundefs {
            let fundef = self.trav_fundef(fundef)?;
            new_program.fundefs.push(fundef);
        }

        Ok(new_program)
    }

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err>;

    fn trav_block(&mut self, block: Block<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Block<Self::OutAst>, Self::Err>;

    fn trav_ssa(&mut self, id: ArgOrVar, _fundef: &mut Fundef<Self::InAst>) -> Result<ArgOrVar, Self::Err>;

    fn trav_expr(&mut self, expr: Expr<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Expr<Self::OutAst>, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n, fundef).map(Tensor),
            Binary(n) => self.trav_binary(n, fundef).map(Binary),
            Unary(n) => self.trav_unary(n, fundef).map(Unary),
            Bool(n) => self.trav_bool(n, fundef).map(Bool),
            U32(n) => self.trav_u32(n, fundef).map(U32),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Tensor<Self::OutAst>, Self::Err> {
        let expr = self.trav_block(tensor.body, fundef)?;
        let iv = self.trav_iv(tensor.iv, fundef)?;
        let lb = self.trav_ssa(tensor.lb, fundef)?;
        let ub = self.trav_ssa(tensor.ub, fundef)?;
        Ok(Tensor { iv, body: expr, lb, ub })
    }

    fn trav_iv(&mut self, iv: IndexVector, fundef: &mut Fundef<Self::InAst>) -> Result<IndexVector, Self::Err>;

    fn trav_binary(&mut self, binary: Binary, fundef: &mut Fundef<Self::InAst>) -> Result<Binary, Self::Err> {
        let l = self.trav_ssa(binary.l, fundef)?;
        let r = self.trav_ssa(binary.r, fundef)?;
        Ok(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary, fundef: &mut Fundef<Self::InAst>) -> Result<Unary, Self::Err> {
        let r = self.trav_ssa(unary.r, fundef)?;
        Ok(Unary { r, op: unary.op })
    }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef<Self::InAst>) -> Result<bool, Self::Err> {
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef<Self::InAst>) -> Result<u32, Self::Err> {
        Ok(value)
    }
}

pub trait Traversal<Ast: AstConfig> {
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
        for arg in &mut fundef.args {
            self.trav_farg(arg)?;
        }
        self.trav_block(&mut fundef.block)?;
        Self::DEFAULT
    }

    fn trav_block(&mut self, block: &mut Block<Ast>) -> Result<Self::Ok, Self::Err> {
        self.trav_ssa(&mut block.ret)?;
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
        self.trav_block(&mut tensor.body)?;
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
