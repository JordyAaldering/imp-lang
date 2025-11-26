use crate::ast::*;

pub trait Traversal {
    type Ok;

    type Err;

    const OK: Self::Ok;

    fn trav_fundef(&mut self, fundef: &mut Fundef) -> Result<Self::Ok, Self::Err> {
        for farg in &mut fundef.args {
            self.trav_farg(farg)?;
        }

        self.trav_identifier(&mut fundef.ret_id)?;

        Ok(Self::OK)
    }

    fn trav_farg(&mut self, _farg: &mut Avis) -> Result<Self::Ok, Self::Err> {
        Ok(Self::OK)
    }

    fn trav_identifier(&mut self, id: &mut ArgOrVar) -> Result<Self::Ok, Self::Err>;

    fn trav_expr(&mut self, expr: &mut Expr) -> Result<Self::Ok, Self::Err> {
        match expr {
            Expr::Binary(n) => self.trav_binary(n),
            Expr::Unary(n) => self.trav_unary(n),
            Expr::Bool(n) => self.trav_bool(n),
            Expr::U32(n) => self.trav_u32(n),
        }
    }

    fn trav_binary(&mut self, binary: &mut Binary) -> Result<Self::Ok, Self::Err> {
        self.trav_identifier(&mut binary.l)?;
        self.trav_identifier(&mut binary.r)?;
        Ok(Self::OK)
    }

    fn trav_unary(&mut self, unary: &mut Unary) -> Result<Self::Ok, Self::Err> {
        self.trav_identifier(&mut unary.r)?;
        Ok(Self::OK)
    }

    fn trav_bool(&mut self, _value: &mut bool) -> Result<Self::Ok, Self::Err> {
        Ok(Self::OK)
    }

    fn trav_u32(&mut self, _value: &mut u32) -> Result<Self::Ok, Self::Err> {
        Ok(Self::OK)
    }
}