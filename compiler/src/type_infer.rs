use std::{collections::HashMap, mem};

use slotmap::{SecondaryMap, SlotMap};

use crate::{ast::*, traverse::Rewriter};

pub fn type_infer(program: Program<UntypedAst>) -> Result<Program<TypedAst>, InferenceError> {
    let mut fundefs = Vec::new();

    for fundef in program.fundefs {
        let (_, res) = TypeInfer::new().trav_fundef(fundef)?;
        fundefs.push(res);
    }

    Ok(Program { fundefs })
}

pub struct TypeInfer {
    args: Vec<Avis<UntypedAst>>,
    old_ids: SlotMap<UntypedKey, Avis<UntypedAst>>,
    scopes: Vec<SecondaryMap<UntypedKey, Expr<UntypedAst>>>,
    new_ids: SlotMap<TypedKey, Avis<TypedAst>>,
    new_ssa: Vec<SecondaryMap<TypedKey, Expr<TypedAst>>>,
    keymap: HashMap<UntypedKey, TypedKey>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            old_ids: SlotMap::with_key(),
            scopes: Vec::new(),
            new_ids: SlotMap::with_key(),
            new_ssa: Vec::new(),
            keymap: HashMap::new(),
        }
    }

    fn find_ssa(&self, key: UntypedKey) -> &Expr<UntypedAst> {
        for scope in self.scopes.iter().rev() {
            if let Some(expr) = scope.get(key) {
                return expr;
            }
        }
        unreachable!()
    }
}

impl Rewriter for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Ok = Type;

    type Err = InferenceError;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<(Type, Fundef<Self::OutAst>), InferenceError> {
        self.args = fundef.args.clone();
        self.old_ids = fundef.ids.clone();
        self.scopes.push(fundef.ssa.clone());
        self.new_ssa.push(SecondaryMap::new());

        let (ret_ty, ret) = self.trav_ssa(fundef.ret)?;

        let mut new_args = Vec::new();
        for (i, arg) in self.args.iter().enumerate() {
            new_args.push(Avis {
                key: ArgOrVar::Arg(i),
                name: arg.name.clone(),
                ty: arg.ty.clone().unwrap(),
            });
        }
        self.args.clear();

        let mut ids = SlotMap::with_key();
        mem::swap(&mut self.new_ids, &mut ids);
        self.scopes.pop().unwrap();
        assert!(self.scopes.is_empty());
        let ssa = self.new_ssa.pop().unwrap();
        assert!(self.new_ssa.is_empty());

        Ok((ret_ty, Fundef {
            name: fundef.name,
            args: new_args,
            ids,
            ssa,
            ret,
        }))
    }

    fn trav_ssa(&mut self, id: ArgOrVar<Self::InAst>) -> Result<(Type, ArgOrVar<Self::OutAst>), InferenceError> {
        match id {
            ArgOrVar::Arg(i) => {
                let ty = self.args[i].ty.clone().unwrap();
                Ok((ty, ArgOrVar::Arg(i)))
            },
            ArgOrVar::Var(old_key) => {
                let old_expr = self.find_ssa(old_key).clone();
                let (new_ty, new_expr) = self.trav_expr(old_expr)?;

                let old_avis = &self.old_ids[old_key];
                let new_key = self.new_ids.insert_with_key(|key| {
                    Avis {
                        key: ArgOrVar::Var(key),
                        name: old_avis.name.clone(),
                        ty: new_ty.clone(),
                    }
                });
                self.keymap.insert(old_key, new_key);
                self.new_ssa.last_mut().unwrap().insert(new_key, new_expr);

                Ok((new_ty, ArgOrVar::Var(new_key)))
            },
            ArgOrVar::Iv(k) => {
                let ty = Type::scalar(BaseType::U32);
                let k = self.keymap[&k];
                Ok((ty, ArgOrVar::Iv(k)))
            },
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<(Type, Tensor<Self::OutAst>), InferenceError> {
        self.scopes.push(tensor.ssa.clone());
        self.new_ssa.push(SecondaryMap::new());

        let (_, lb) = self.trav_ssa(tensor.lb)?;
        let (_, ub) = self.trav_ssa(tensor.ub)?;

        let old_avis = &self.old_ids[tensor.iv];
        let iv_new_key = self.new_ids.insert_with_key(|key| {
            Avis {
                key: ArgOrVar::Iv(key),
                name: old_avis.name.clone(),
                ty: Type::scalar(BaseType::U32),
            }
        });
        self.keymap.insert(tensor.iv, iv_new_key);

        let (ret_ty, ret) = self.trav_ssa(tensor.ret)?;

        let shp = if let Shape::Scalar = ret_ty.shp { "." } else { "*" };
        let tensor_ty = Type::vector(ret_ty.basetype, shp);

        self.scopes.pop().unwrap();
        let ssa = self.new_ssa.pop().unwrap();

        Ok((tensor_ty, Tensor {
            iv: iv_new_key,
            lb,
            ub,
            ret,
            ssa,
        }))
    }

    fn trav_binary(&mut self, binary: Binary<Self::InAst>) -> Result<(Type, Binary<Self::OutAst>), Self::Err> {
        let (lty, l) = self.trav_ssa(binary.l)?;
        let (rty, r) = self.trav_ssa(binary.r)?;

        let ty = unifies(lty, rty)?;

        use Bop::*;
        let ty = match binary.op {
            Add | Sub | Mul | Div => {
                // TODO: check if unifies with num
                Type { basetype: BaseType::U32, shp: ty.shp }
            },
            Eq | Ne => {
                Type { basetype: BaseType::Bool, shp: ty.shp }
            },
            Lt | Le | Gt | Ge => {
                // TODO: check if unifies with num
                Type { basetype: BaseType::Bool, shp: ty.shp }
            },
        };

        Ok((ty, Binary { l, r, op: binary.op }))
    }

    fn trav_unary(&mut self, unary: Unary<Self::InAst>) -> Result<(Type, Unary<Self::OutAst>), Self::Err> {
        let (rty, r) = self.trav_ssa(unary.r)?;

        use Uop::*;
        let ty = match unary.op {
            Neg => {
                // TODO: check if r_ty unifies with signed num
                Type { basetype: BaseType::U32, shp: rty.shp }
            },
            Not => {
                // TODO: check if r_ty unifies with bool
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
        };

        Ok((ty, Unary { r, op: unary.op }))
    }

    fn trav_bool(&mut self, value: bool) -> Result<(Type, bool), Self::Err> {
        let ty = Type::scalar(BaseType::Bool);
        Ok((ty, value))
    }

    fn trav_u32(&mut self, value: u32) -> Result<(Type, u32), Self::Err> {
        let ty = Type::scalar(BaseType::U32);
        Ok((ty, value))
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    // TODO
    Ok(a)
}
