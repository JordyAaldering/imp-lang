use crate::ast::*;

pub struct TypeInfer;

#[derive(Debug)]
pub enum InferenceError {}

type InferenceResult<T> = Result<T, InferenceError>;

impl TypeInfer {
    pub fn new() -> Self {
        Self
    }

    pub fn infer_program(&mut self, program: Program) -> InferenceResult<Program> {
        let mut fundefs = Vec::new();
        for f in program.fundefs {
            fundefs.push(self.infer_fundef(f)?);
        }

        Ok(Program { fundefs })
    }

    pub fn infer_fundef(&mut self, fundef: Fundef) -> InferenceResult<Fundef> {
        let mut new_fundef = fundef.clone();

        // Go bottom-up using the return value to infer all types
        let _ret_value = self.infer_type(&mut new_fundef, fundef.ret_id);
        // todo: check if matches user given return type

        Ok(new_fundef)
    }

    pub fn infer_type(&mut self, scope: &mut Fundef, aov: ArgOrVar) -> Type {
        match aov {
            ArgOrVar::Arg(i) => {
                scope.args[i].ty.unwrap()
            },
            ArgOrVar::Var(k) => {
                let ty = match scope.ssa[k] {
                    Expr::Binary(Binary { l, r, op }) => {
                        let _l_ty = self.infer_type(scope, l);
                        let _r_ty = self.infer_type(scope, r);
                        // TODO: check if l_ty and r_ty unify

                        use Bop::*;
                        match op {
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
                        }
                    },
                    Expr::Unary(Unary { r, op }) => {
                        let _r_ty = self.infer_type(scope, r);

                        use Uop::*;
                        match op {
                            Neg => {
                                // TODO: check if r_ty unifies with num
                                Type::U32
                            },
                            Not => {
                                // TODO: check if r_ty unifies with bool
                                Type::Bool
                            },
                        }
                    },
                    Expr::Bool(_) => {
                        Type::Bool
                    },
                    Expr::U32(_) => {
                        Type::U32
                    },
                };

                scope.vars[k].ty = Some(ty);
                ty
            }
        }
    }
}
