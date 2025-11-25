use std::collections::HashMap;

use slotmap::SlotMap;

use crate::ast::*;

pub struct TypeInfer {
    vars: Option<SlotMap<VarKey, VarInfo<TypedAst>>>,
    varkey_rename: HashMap<VarKey, VarKey>,
}

#[derive(Debug)]
pub enum InferenceError {}

type InferenceResult<T> = Result<T, InferenceError>;

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            vars: None,
            varkey_rename: HashMap::new(),
        }
    }

    pub fn infer_program(&mut self, program: Program<UntypedAst>) -> InferenceResult<Program<TypedAst>> {
        let mut fundefs = Vec::new();
        for f in program.fundefs {
            fundefs.push(self.infer_fundef(f)?);
        }

        Ok(Program { fundefs })
    }

    pub fn infer_fundef(&mut self, fundef: Fundef<UntypedAst>) -> InferenceResult<Fundef<TypedAst>> {
        self.vars = Some(SlotMap::with_key());
        self.varkey_rename.clear();

        for old_key in &fundef.args {
            let arg_info = &fundef.vars[*old_key];
            let arg_ty = arg_info.ty.expect("formal arguments can never be untyped");
            let new_key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                VarInfo {
                    key,
                    id: arg_info.id.clone(),
                    ty: arg_ty,
                }
            });
            self.varkey_rename.insert(*old_key, new_key);
        }

        // Insert return type as well
        let ret_value = {
            let ret_info = &fundef.vars[fundef.ret_value];
            let ret_ty = ret_info.ty.expect("return value can never be untyped");
            let new_key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                VarInfo {
                    key,
                    id: ret_info.id.clone(),
                    ty: ret_ty,
                }
            });
            self.varkey_rename.insert(fundef.ret_value, new_key);
            new_key
        };

        // Go bottom-up using the return value to infer all types
        self.infer_type(&fundef, fundef.ret_value);

        Ok(Fundef {
            id: fundef.id,
            args: fundef.args,
            vars: self.vars.take().unwrap(),
            ssa: fundef.ssa,
            ret_value,
        })
    }

    pub fn infer_type(&mut self, scope: &Fundef<UntypedAst>, varkey: VarKey) -> Type {
        let old_varinfo = &scope.vars[varkey];

        if let Some(expr) = scope.ssa.get(varkey) {
            match expr {
                Expr::Binary(Binary { l, r, op }) => {
                    let _l_ty = self.infer_type(scope, *l);
                    let _r_ty = self.infer_type(scope, *r);
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
                    let _r_ty = self.infer_type(scope, *r);

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
                    let new_key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                        VarInfo {
                            key,
                            id: old_varinfo.id.clone(),
                            ty: Type::Bool,
                        }
                    });
                    self.varkey_rename.insert(varkey, new_key);
                    Type::Bool
                },
                Expr::U32(_) => {
                    let new_key = self.vars.as_mut().unwrap().insert_with_key(|key| {
                        VarInfo {
                            key,
                            id: old_varinfo.id.clone(),
                            ty: Type::U32,
                        }
                    });
                    self.varkey_rename.insert(varkey, new_key);
                    Type::U32
                },
            }
        } else {
            // No expression exists, so this must be an argument
            let argkey = self.varkey_rename[&varkey];
            self.vars.as_mut().unwrap()[argkey].ty
        }
    }
}