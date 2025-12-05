use std::mem;

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

    fn trav_expr(&mut self, expr: Expr, fundef: &mut Fundef<Self::InAst>) -> Result<Expr, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n, fundef).map(Tensor),
            Binary(n) => self.trav_binary(n, fundef).map(Binary),
            Unary(n) => self.trav_unary(n, fundef).map(Unary),
            Bool(n) => self.trav_bool(n, fundef).map(Bool),
            U32(n) => self.trav_u32(n, fundef).map(U32),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor, fundef: &mut Fundef<Self::InAst>) -> Result<Tensor, Self::Err> {
        let iv = self.trav_iv(tensor.iv, fundef)?;
        let expr = self.trav_ssa(tensor.expr, fundef)?;
        let lb = self.trav_ssa(tensor.lb, fundef)?;
        let ub = self.trav_ssa(tensor.ub, fundef)?;
        Ok(Tensor { iv, expr, lb, ub })
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
    type Err;

    fn trav_program(&mut self, mut program: Program<Ast>) -> Result<Program<Ast>, Self::Err> {
        let mut old_fundefs = Vec::new();
        mem::swap(&mut program.fundefs, &mut old_fundefs);
        for fundef in old_fundefs {
            let fundef = self.trav_fundef(fundef)?;
            program.fundefs.push(fundef);
        }

        Ok(program)
    }

    fn trav_fundef(&mut self, mut fundef: Fundef<Ast>) -> Result<Fundef<Ast>, Self::Err> {
        let mut old_args = Vec::new();
        mem::swap(&mut fundef.args, &mut old_args);
        for farg in old_args {
            let farg = self.trav_farg(farg, &fundef)?;
            fundef.args.push(farg);
        }

        fundef.block = self.trav_block(fundef.block.clone(), &fundef)?;

        Ok(fundef)
    }

    fn trav_block(&mut self, mut block: Block<Ast>, fundef: &Fundef<Ast>) -> Result<Block<Ast>, Self::Err> {
        let old_ret = block.ret.clone();
        block.ret = self.trav_ssa(old_ret, &fundef)?;

        Ok(block)
    }

    fn trav_farg(&mut self, farg: Avis<Ast>, _fundef: &Fundef<Ast>) -> Result<Avis<Ast>, Self::Err> {
        Ok(farg)
    }

    fn trav_ssa(&mut self, id: ArgOrVar, _fundef: &Fundef<Ast>) -> Result<ArgOrVar, Self::Err> {
        Ok(id)
    }

    fn trav_expr(&mut self, expr: Expr, fundef: &Fundef<Ast>) -> Result<Expr, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n, fundef).map(Tensor),
            Binary(n) => self.trav_binary(n, fundef).map(Binary),
            Unary(n) => self.trav_unary(n, fundef).map(Unary),
            Bool(n) => self.trav_bool(n, fundef).map(Bool),
            U32(n) => self.trav_u32(n, fundef).map(U32),
        }
    }

    fn trav_tensor(&mut self, mut tensor: Tensor, fundef: &Fundef<Ast>) -> Result<Tensor, Self::Err> {
        tensor.expr = self.trav_ssa(tensor.expr, fundef)?;
        Ok(tensor)
    }

    fn trav_binary(&mut self, mut binary: Binary, fundef: &Fundef<Ast>) -> Result<Binary, Self::Err> {
        binary.l = self.trav_ssa(binary.l, fundef)?;
        binary.r = self.trav_ssa(binary.r, fundef)?;
        Ok(binary)
    }

    fn trav_unary(&mut self, mut unary: Unary, fundef: &Fundef<Ast>) -> Result<Unary, Self::Err> {
        unary.r = self.trav_ssa(unary.r, fundef)?;
        Ok(unary)
    }

    fn trav_bool(&mut self, value: bool, _fundef: &Fundef<Ast>) -> Result<bool, Self::Err> {
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &Fundef<Ast>) -> Result<u32, Self::Err> {
        Ok(value)
    }
}
