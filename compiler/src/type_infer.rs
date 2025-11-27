use std::{collections::HashMap, mem};

use slotmap::{Key, SecondaryMap, SlotMap};

use crate::{ast::*, traverse::Rewriter};

pub struct TypeInfer {
    new_vars: SlotMap<TypedKey, Avis<TypedAst>>,
    new_ssa: SecondaryMap<TypedKey, Expr<TypedAst>>,
    iv_rename: HashMap<UntypedKey, TypedKey>,
    found_ty: Option<Type>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            new_vars: SlotMap::with_key(),
            new_ssa: SecondaryMap::new(),
            iv_rename: HashMap::new(),
            found_ty: None,
        }
    }
}

impl Rewriter for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Err = InferenceError;

    fn trav_fundef(&mut self, mut fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err> {
        self.iv_rename.clear();

        let mut new_fundef = Fundef {
            name: fundef.name.to_owned(),
            args: Vec::new(),
            vars: SlotMap::with_key(),
            ssa: SecondaryMap::new(),
            ret: ArgOrVar::Var(TypedKey::null()),
        };

        for (i, arg) in fundef.args.iter().enumerate() {
            let k = ArgOrVar::Arg(i);
            new_fundef.args.push(Avis::new(k, &arg.name, arg.ty.clone().unwrap()));
        }

        let old_key = fundef.ret.clone();
        new_fundef.ret = self.trav_ssa(old_key, &mut fundef)?;

        mem::swap(&mut self.new_vars, &mut new_fundef.vars);

        mem::swap(&mut self.new_ssa, &mut new_fundef.ssa);

        Ok(new_fundef)
    }

    fn trav_ssa(&mut self, id: ArgOrVar<UntypedAst>, fundef: &mut Fundef<Self::InAst>) -> Result<ArgOrVar<TypedAst>, Self::Err> {
        let id = match id {
            ArgOrVar::Arg(i) => {
                let ty = fundef.args[i].ty.clone().expect("function argument cannot be untyped");
                self.found_ty = Some(ty);
                ArgOrVar::Arg(i)
            },
            ArgOrVar::Var(old_key) => {
                let new_expr = self.trav_expr(fundef.ssa[old_key].clone(), fundef)?;

                let old_avis = &fundef.vars[old_key];
                let new_key = self.new_vars.insert_with_key(|new_key| {
                    Avis { name: old_avis.name.to_owned(), ty: self.found_ty.clone().unwrap(), _key: ArgOrVar::Var(new_key) }
                });
                println!("replaced {:?} by {:?} = {:?}", old_key, new_key, new_expr);
                self.new_ssa.insert(new_key, new_expr);
                ArgOrVar::Var(new_key)
            },
            ArgOrVar::IV(old_key) => {
                let new_key = self.iv_rename[&old_key];
                println!("renamed index vector {:?} by {:?}", old_key, new_key);
                ArgOrVar::IV(new_key)
            },
        };

        Ok(id)
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Tensor<Self::OutAst>, Self::Err> {
        let iv = self.trav_iv(tensor.iv, fundef)?;
        let lb = self.trav_ssa(tensor.lb, fundef)?;
        let ub = self.trav_ssa(tensor.ub, fundef)?;

        let expr = self.trav_ssa(tensor.expr, fundef)?;
        let ety = self.found_ty.take().unwrap();

        self.found_ty = Some(Type { basetype: ety.basetype, shp: Shape::Vector((if let Shape::Scalar = ety.shp { "." } else { "*" }).to_owned()) });
        Ok(Tensor { iv, expr, lb, ub })
    }

    fn trav_iv(&mut self, iv: IndexVector<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<IndexVector<Self::OutAst>, Self::Err> {
        let old_avis = &fundef.vars[iv.0];
        let new_key = self.new_vars.insert_with_key(|new_key| {
            Avis { name: old_avis.name.to_owned(), ty: Type { basetype: BaseType::U32, shp: Shape::Scalar }, _key: ArgOrVar::IV(new_key) }
        });
        self.iv_rename.insert(iv.0, new_key);
        println!("replaced index vector {:?} by {:?}", iv.0, new_key);
        Ok(IndexVector(new_key))
    }

    fn trav_binary(&mut self, binary: Binary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Binary<Self::OutAst>, Self::Err> {
        let l = self.trav_ssa(binary.l, fundef)?;
        let _lty = self.found_ty.take().unwrap();
        let r = self.trav_ssa(binary.r, fundef)?;
        let rty = self.found_ty.take().unwrap();

        // TODO: check if lty and rty unify

        use Bop::*;
        self.found_ty = Some(match binary.op {
            Add | Sub | Mul | Div => {
                // TODO: check if unifies with num
                Type { basetype: BaseType::U32, shp: rty.shp }
            },
            Eq | Ne => {
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
            Lt | Le | Gt | Ge => {
                // TODO: check if unifies with num
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
        });

        Ok(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Unary<Self::OutAst>, Self::Err> {
        let r = self.trav_ssa(unary.r, fundef)?;
        let rty = self.found_ty.take().unwrap();

        use Uop::*;
        self.found_ty = Some(match unary.op {
            Neg => {
                // TODO: check if r_ty unifies with signed num
                Type { basetype: BaseType::U32, shp: rty.shp }
            },
            Not => {
                // TODO: check if r_ty unifies with bool
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
        });

        Ok(Unary { r, op: unary.op })
    }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef<Self::InAst>) -> Result<bool, Self::Err> {
        self.found_ty = Some(Type { basetype: BaseType::Bool, shp: Shape::Scalar });
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef<Self::InAst>) -> Result<u32, Self::Err> {
        self.found_ty = Some(Type { basetype: BaseType::U32, shp: Shape::Scalar });
        Ok(value)
    }
}
