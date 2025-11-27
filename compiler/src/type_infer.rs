use crate::{ast::*, traverse::Traversal};

pub struct TypeInfer {
    found_ty: Option<Type>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            found_ty: None
        }
    }
}

impl Traversal for TypeInfer {
    type Err = InferenceError;

    fn trav_fundef(&mut self, mut fundef: Fundef) -> Result<Fundef, Self::Err> {
        fundef.ret_id = self.trav_identifier(fundef.ret_id, &mut fundef)?;
        if let ArgOrVar::Var(k) = fundef.ret_id {
            fundef.vars[k].ty = self.found_ty;
        }

        Ok(fundef)
    }

    fn trav_identifier(&mut self, id: ArgOrVar, fundef: &mut Fundef) -> Result<ArgOrVar, Self::Err> {
        match id {
            ArgOrVar::Arg(i) => {
                let ty = fundef.args[i].ty.expect("function argument cannot be untyped");
                self.found_ty = Some(ty);
            },
            ArgOrVar::Var(k) => {
                fundef.ssa[k] = self.trav_expr(fundef.ssa[k].clone(), fundef)?;
                fundef.vars[k].ty = self.found_ty;
                // match expr {
                //     Expr::Binary(n) => {
                //         *n = self.trav_binary(*n, fundef)?;
                //     },
                //     Expr::Unary(n) => {
                //         *n = self.trav_unary(*n, fundef)?;
                //     },
                //     Expr::Bool(_) => {
                //         self.found_ty = Some(Type::Bool);
                //     },
                //     Expr::U32(_) => {
                //         self.found_ty = Some(Type::U32);
                //     },
                // };
            },
        };

        Ok(id)
    }

    fn trav_binary(&mut self, mut binary: Binary, fundef: &mut Fundef) -> Result<Binary, Self::Err> {
        binary.l = self.trav_identifier(binary.l, fundef)?;
        fundef[binary.l].ty = self.found_ty;

        binary.r = self.trav_identifier(binary.r, fundef)?;
        fundef[binary.r].ty = self.found_ty;

        // TODO: check if lty and rty unify

        use Bop::*;
        self.found_ty = Some(match binary.op {
            Add | Sub | Mul | Div => {
                // TODO: check if unifies with num
                Type::U32
            },
            Eq | Ne => {
                Type::Bool
            },
            Lt | Le | Gt | Ge => {
                // TODO: check if unifies with num
                Type::Bool
            },
        });

        Ok(binary)
    }

    fn trav_unary(&mut self, mut unary: Unary, fundef: &mut Fundef) -> Result<Unary, Self::Err> {
        unary.r = self.trav_identifier(unary.r, fundef)?;
        fundef[unary.r].ty = self.found_ty;

        use Uop::*;
        self.found_ty = Some(match unary.op {
            Neg => {
                // TODO: check if r_ty unifies with signed num
                Type::U32
            },
            Not => {
                // TODO: check if r_ty unifies with bool
                Type::Bool
            },
        });

        Ok(unary)
    }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef) -> Result<bool, Self::Err> {
        self.found_ty = Some(Type::Bool);
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef) -> Result<u32, Self::Err> {
        self.found_ty = Some(Type::U32);
        Ok(value)
    }
}
