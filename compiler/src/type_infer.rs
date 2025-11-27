use std::{collections::HashMap, mem};

use slotmap::{Key, SecondaryMap, SlotMap};

use crate::{ast::*, traverse::Traversal};

pub struct TypeInfer {
    key_rename: HashMap<UntypedKey, TypedKey>,
    new_vars: SlotMap<TypedKey, Avis<TypedAst>>,
    new_ssa: SecondaryMap<TypedKey, Expr<TypedAst>>,
    found_ty: Option<Type>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            key_rename: HashMap::new(),
            new_vars: SlotMap::with_key(),
            new_ssa: SecondaryMap::new(),
            found_ty: None,
        }
    }
}

impl Traversal for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Err = InferenceError;

    fn trav_fundef(&mut self, mut fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err> {
        debug_assert!(self.key_rename.is_empty());

        let mut new_fundef = Fundef {
            name: fundef.name.to_owned(),
            args: Vec::new(),
            vars: SlotMap::with_key(),
            ssa: SecondaryMap::new(),
            ret_id: ArgOrVar::Var(TypedKey::null()),
        };

        for (i, arg) in fundef.args.iter().enumerate() {
            let k = ArgOrVar::Arg(i);
            new_fundef.args.push(Avis::new(k, &arg.name, arg.ty.unwrap()));
        }

        let old_key = fundef.ret_id.clone();
        new_fundef.ret_id = self.trav_identifier(old_key, &mut fundef)?;

        mem::swap(&mut self.new_vars, &mut new_fundef.vars);

        mem::swap(&mut self.new_ssa, &mut new_fundef.ssa);

        self.key_rename.clear();

        Ok(new_fundef)
    }

    fn trav_identifier(&mut self, id: ArgOrVar<UntypedAst>, fundef: &mut Fundef<Self::InAst>) -> Result<ArgOrVar<TypedAst>, Self::Err> {
        let id = match id {
            ArgOrVar::Arg(i) => {
                let ty = fundef.args[i].ty.expect("function argument cannot be untyped");
                self.found_ty = Some(ty);
                ArgOrVar::Arg(i)
            },
            ArgOrVar::Var(old_key) => {
                let new_expr = self.trav_expr(fundef.ssa[old_key].clone(), fundef)?;

                let old_avis = &fundef.vars[old_key];
                let new_key = self.new_vars.insert_with_key(|new_key| {
                    Avis { name: old_avis.name.to_owned(), ty: self.found_ty.unwrap(), _key: ArgOrVar::Var(new_key) }
                });
                println!("replaced {:?} by {:?} = {:?}", old_key, new_key, new_expr);
                self.new_ssa.insert(new_key, new_expr);
                self.key_rename.insert(old_key, new_key);
                ArgOrVar::Var(new_key)
            },
        };

        Ok(id)
    }

    fn trav_binary(&mut self, binary: Binary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Binary<Self::OutAst>, Self::Err> {
        let l = self.trav_identifier(binary.l, fundef)?;
        let r = self.trav_identifier(binary.r, fundef)?;

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

        Ok(Binary { l, r, op: binary.op })
    }

    fn trav_unary(&mut self, unary: Unary<Self::InAst>, fundef: &mut Fundef<Self::InAst>) -> Result<Unary<Self::OutAst>, Self::Err> {
        let r = self.trav_identifier(unary.r, fundef)?;

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

        Ok(Unary { r, op: unary.op })
    }

    fn trav_bool(&mut self, value: bool, _fundef: &mut Fundef<Self::InAst>) -> Result<bool, Self::Err> {
        self.found_ty = Some(Type::Bool);
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32, _fundef: &mut Fundef<Self::InAst>) -> Result<u32, Self::Err> {
        self.found_ty = Some(Type::U32);
        Ok(value)
    }
}
