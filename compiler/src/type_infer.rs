use std::collections::HashMap;

use crate::{ast::*, traverse::Rewriter};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    let mut fundefs = Vec::new();

    for fundef in program.fundefs {
        let (_, out) = TypeInfer::new().trav_fundef(fundef)?;
        fundefs.push(out);
    }

    Ok(Program { fundefs })
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Avis<UntypedAst>>,
    scopes: Vec<SsaBlock<'ast, UntypedAst>>,
    idmap: HashMap<*const Avis<UntypedAst>, &'ast Avis<TypedAst>>,
    new_ids: Vec<&'ast Avis<TypedAst>>,
    new_ssa: Vec<SsaBlock<'ast, TypedAst>>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl<'ast> TypeInfer<'ast> {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            scopes: Vec::new(),
            idmap: HashMap::new(),
            new_ids: Vec::new(),
            new_ssa: Vec::new(),
        }
    }

    fn alloc_avis(&self, name: String, ty: Type) -> &'ast Avis<TypedAst> {
        Box::leak(Box::new(Avis { name, ty }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        Box::leak(Box::new(expr))
    }

    fn find_local_def(&self, key: &'ast Avis<UntypedAst>) -> LocalDef<'ast, UntypedAst> {
        for scope in self.scopes.iter().rev() {
            for entry in scope.iter().rev() {
                if std::ptr::eq(entry.avis(), key) {
                    return entry.def();
                }
            }
        }
        unreachable!()
    }
}

impl<'ast> Rewriter<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;
    type OutAst = TypedAst;
    type Ok = Type;
    type Err = InferenceError;

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Result<(Type, Fundef<'ast, Self::OutAst>), InferenceError> {
        self.args = fundef.args.clone();
        self.scopes.push(fundef.ssa.clone());
        self.new_ssa.push(Vec::new());
        self.idmap.clear();
        self.new_ids.clear();

        let (ret_ty, ret) = self.trav_ssa(fundef.ret)?;

        let new_args = self.args.iter().map(|arg| {
            self.alloc_avis(arg.name.clone(), arg.ty.clone().unwrap())
        }).collect::<Vec<_>>();

        self.scopes.pop().unwrap();
        let ssa = self.new_ssa.pop().unwrap();

        Ok((ret_ty, Fundef {
            name: fundef.name,
            args: new_args,
            ids: self.new_ids.clone(),
            ssa,
            ret,
        }))
    }

    fn trav_ssa(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> Result<(Type, ArgOrVar<'ast, Self::OutAst>), InferenceError> {
        match id {
            ArgOrVar::Arg(i) => {
                let ty = self.args[i].ty.clone().unwrap();
                Ok((ty, ArgOrVar::Arg(i)))
            }
            ArgOrVar::Var(old) => {
                if let Some(new_id) = self.idmap.get(&(old as *const _)) {
                    return Ok((new_id.ty.clone(), ArgOrVar::Var(*new_id)));
                }

                match self.find_local_def(old) {
                    LocalDef::Assign(old_expr) => {
                        let (new_ty, new_expr) = self.trav_expr(old_expr.clone())?;

                        let new_id = self.alloc_avis(old.name.clone(), new_ty.clone());
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);

                        let expr_ref = self.alloc_expr(new_expr);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Assign {
                            avis: new_id,
                            expr: expr_ref,
                        });

                        Ok((new_ty, ArgOrVar::Var(new_id)))
                    }
                    LocalDef::IndexRange { lb, ub } => {
                        let (_, lb) = self.trav_ssa(lb)?;
                        let (_, ub) = self.trav_ssa(ub)?;

                        let new_id = self.alloc_avis(old.name.clone(), Type::scalar(BaseType::U32));
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Index {
                            avis: new_id,
                            lb,
                            ub,
                        });

                        Ok((Type::scalar(BaseType::U32), ArgOrVar::Var(new_id)))
                    }
                }
            }
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Result<(Type, Tensor<'ast, Self::OutAst>), InferenceError> {
        self.scopes.push(tensor.ssa.clone());
        self.new_ssa.push(Vec::new());

        let (_, lb) = self.trav_ssa(tensor.lb)?;
        let (_, ub) = self.trav_ssa(tensor.ub)?;

        let iv_new = self.alloc_avis(tensor.iv.name.clone(), Type::scalar(BaseType::U32));
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);
        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Index {
            avis: iv_new,
            lb,
            ub,
        });

        let (ret_ty, ret) = self.trav_ssa(tensor.ret)?;
        let shp = if let Shape::Scalar = ret_ty.shp { "." } else { "*" };
        let tensor_ty = Type::vector(ret_ty.basetype, shp);

        self.scopes.pop().unwrap();
        let ssa = self.new_ssa.pop().unwrap();

        Ok((tensor_ty, Tensor { iv: iv_new, lb, ub, ret, ssa }))
    }

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Result<(Type, Binary<'ast, Self::OutAst>), Self::Err> {
        let (lty, l) = self.trav_ssa(binary.l)?;
        let (rty, r) = self.trav_ssa(binary.r)?;

        let ty = unifies(lty, rty)?;

        use Bop::*;
        let ty = match binary.op {
            Add | Sub | Mul | Div => Type { basetype: BaseType::U32, shp: ty.shp },
            Eq | Ne => Type { basetype: BaseType::Bool, shp: ty.shp },
            Lt | Le | Gt | Ge => Type { basetype: BaseType::Bool, shp: ty.shp },
        };

        Ok((ty, Binary { l, r, op: binary.op }))
    }

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Result<(Type, Unary<'ast, Self::OutAst>), Self::Err> {
        let (rty, r) = self.trav_ssa(unary.r)?;

        use Uop::*;
        let ty = match unary.op {
            Neg => Type { basetype: BaseType::U32, shp: rty.shp },
            Not => Type { basetype: BaseType::Bool, shp: rty.shp },
        };

        Ok((ty, Unary { r, op: unary.op }))
    }

    fn trav_bool(&mut self, value: bool) -> Result<(Type, bool), Self::Err> {
        Ok((Type::scalar(BaseType::Bool), value))
    }

    fn trav_u32(&mut self, value: u32) -> Result<(Type, u32), Self::Err> {
        Ok((Type::scalar(BaseType::U32), value))
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    Ok(a)
}
