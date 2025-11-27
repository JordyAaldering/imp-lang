use std::mem;

use crate::ast::*;

pub trait Traversal {
    type Err;

    fn trav_program(&mut self, mut program: Program) -> Result<Program, Self::Err> {
        let mut orig_fundefs = Vec::new();
        mem::swap(&mut orig_fundefs, &mut program.fundefs);
        for fundef in orig_fundefs {
            let fundef = self.trav_fundef(fundef)?;
            program.fundefs.push(fundef);
        }

        Ok(program)
    }

    fn trav_fundef(&mut self, mut fundef: Fundef) -> Result<Fundef, Self::Err> {
        let mut orig_args = Vec::new();
        mem::swap(&mut orig_args, &mut fundef.args);
        for farg in orig_args {
            let arg = self.trav_farg(farg, &mut fundef)?;
            fundef.args.push(arg);
        }

        fundef.ret_id = self.trav_identifier(fundef.ret_id, &mut fundef)?;

        Ok(fundef)
    }

    // Fundef is deliberately readonly. If you want to make changes to the fundef, that should occur in the fundef traversal
    fn trav_farg(&mut self, farg: Avis, _fundef: &mut Fundef) -> Result<Avis, Self::Err> {
        Ok(farg)
    }

    fn trav_identifier(&mut self, id: ArgOrVar, _fundef: &mut Fundef) -> Result<ArgOrVar, Self::Err>;

    fn trav_expr(&mut self, expr: Expr, fundef: &mut Fundef) -> Result<Expr, Self::Err> {
        use Expr::*;
        let expr = match expr {
            Binary(n) => Binary(self.trav_binary(n, fundef)?),
            Unary(n) => Unary(self.trav_unary(n, fundef)?),
            Bool(n) => Bool(self.trav_bool(n, fundef)?),
            U32(n) => U32(self.trav_u32(n, fundef)?),
        };
        Ok(expr)
    }

    fn trav_binary(&mut self, mut binary: Binary, fundef: &mut Fundef) -> Result<Binary, Self::Err> {
        binary.l = self.trav_identifier(binary.l, fundef)?;
        binary.r = self.trav_identifier(binary.r, fundef)?;
        Ok(binary)
    }

    fn trav_unary(&mut self, mut unary: Unary, fundef: &mut Fundef) -> Result<Unary, Self::Err> {
        unary.r = self.trav_identifier(unary.r, fundef)?;
        Ok(unary)
    }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef) -> Result<bool, Self::Err> {
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef) -> Result<u32, Self::Err> {
        Ok(value)
    }
}