use crate::ast::*;

pub trait Traversal {
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

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err>; /*{
        let mut new_fundef = Fundef {
            name: fundef.name.to_owned(),
            args: Vec::new(),
            vars: SlotMap::with_key(),
            ssa: SecondaryMap::new(),
            ret_id: fundef.ret_id,
        };

        for farg in fundef.args {
            let arg = self.trav_farg(farg, &mut new_fundef)?;
            new_fundef.args.push(arg);
        }

        //fundef.ret_id = self.trav_identifier(fundef.ret_id, &mut fundef)?;

        Ok(new_fundef)
    }*/

    /*fn trav_farg(&mut self, farg: Avis<Self::InAst>, _fundef: &mut Fundef<Self::InAst>) -> Result<Avis<Self::OutAst>, Self::Err> {
        Ok(farg)
    }*/

    fn trav_identifier(&mut self, id: ArgOrVar<Self::InAst>, _fundef: &mut Fundef<Self::InAst>) -> Result<ArgOrVar<Self::OutAst>, Self::Err>;

    fn trav_expr(&mut self, expr: Expr<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Expr<Self::OutAst>, Self::Err> {
        use Expr::*;
        let expr = match expr {
            Binary(n) => Binary(self.trav_binary(n, fundef)?),
            Unary(n) => Unary(self.trav_unary(n, fundef)?),
            Bool(n) => Bool(self.trav_bool(n, fundef)?),
            U32(n) => U32(self.trav_u32(n, fundef)?),
        };
        Ok(expr)
    }

    fn trav_binary(&mut self, binary: Binary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Binary<Self::OutAst>, Self::Err>;
    //     binary.l = self.trav_identifier(binary.l, fundef)?;
    //     binary.r = self.trav_identifier(binary.r, fundef)?;
    //     Ok(binary)
    // }

    fn trav_unary(&mut self, unary: Unary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Unary<Self::OutAst>, Self::Err>;
    //     unary.r = self.trav_identifier(unary.r, fundef)?;
    //     Ok(unary)
    // }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef<Self::InAst>) -> Result<bool, Self::Err> {
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef<Self::InAst>) -> Result<u32, Self::Err> {
        Ok(value)
    }
}