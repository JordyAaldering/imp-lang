use std::mem;

use crate::ast::*;

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

    fn trav_ssa(&mut self, id: ArgOrVar<Self::InAst>, _fundef: &mut Fundef<Self::InAst>) -> Result<ArgOrVar<Self::OutAst>, Self::Err>;

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
        let expr = self.trav_ssa(tensor.expr, fundef)?;
        Ok(Tensor { expr, iv: tensor.iv, lb: tensor.lb, ub: tensor.ub })
    }

    fn trav_binary(&mut self, binary: Binary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Binary<Self::OutAst>, Self::Err> {
        let l = self.trav_ssa(binary.l, fundef)?;
        let r = self.trav_ssa(binary.r, fundef)?;
        Ok(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Unary<Self::OutAst>, Self::Err> {
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

        let old_ret = fundef.ret.clone();
        fundef.ret = self.trav_ssa(old_ret, &fundef)?;

        Ok(fundef)
    }

    fn trav_farg(&mut self, farg: Avis<Ast>, _fundef: &Fundef<Ast>) -> Result<Avis<Ast>, Self::Err> {
        Ok(farg)
    }

    fn trav_ssa(&mut self, id: ArgOrVar<Ast>, _fundef: &Fundef<Ast>) -> Result<ArgOrVar<Ast>, Self::Err> {
        Ok(id)
    }

    fn trav_expr(&mut self, expr: Expr<Ast>, fundef: &Fundef<Ast>) -> Result<Expr<Ast>, Self::Err> {
        use Expr::*;
        match expr {
            Tensor(n) => self.trav_tensor(n, fundef).map(Tensor),
            Binary(n) => self.trav_binary(n, fundef).map(Binary),
            Unary(n) => self.trav_unary(n, fundef).map(Unary),
            Bool(n) => self.trav_bool(n, fundef).map(Bool),
            U32(n) => self.trav_u32(n, fundef).map(U32),
        }
    }

    fn trav_tensor(&mut self, mut tensor: Tensor<Ast>, fundef: &Fundef<Ast>) -> Result<Tensor<Ast>, Self::Err> {
        tensor.expr = self.trav_ssa(tensor.expr, fundef)?;
        Ok(tensor)
    }

    fn trav_binary(&mut self, mut binary: Binary<Ast>, fundef: &Fundef<Ast>) -> Result<Binary<Ast>, Self::Err> {
        binary.l = self.trav_ssa(binary.l, fundef)?;
        binary.r = self.trav_ssa(binary.r, fundef)?;
        Ok(binary)
    }

    fn trav_unary(&mut self, mut unary: Unary<Ast>, fundef: &Fundef<Ast>) -> Result<Unary<Ast>, Self::Err> {
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
