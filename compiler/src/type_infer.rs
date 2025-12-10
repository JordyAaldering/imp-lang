use std::mem;

use crate::{arena::{Arena, Key, SecondaryArena}, ast::*, traverse::{Scoped, Rewriter}};

pub struct TypeInfer {
    // todo: this should be a vec of arenas, one for each scoping level
    new_vars: Arena<Avis<TypedAst>>,
    new_ssa: SecondaryArena<Expr<TypedAst>>,
    // todo: this should be included in the return type, probably we should
    // return Result<(Self::OK, Node), Self::Err> instead
    found_ty: Option<Type>,
    args: Vec<Avis<TypedAst>>,
    scopes: Vec<Block<UntypedAst>>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            new_vars: Arena::new(),
            new_ssa: SecondaryArena::new(),
            found_ty: None,
            args: Vec::new(),
            scopes: Vec::new(),
        }
    }
}

impl Scoped<UntypedAst> for TypeInfer {
    fn find_id(&self, key: Key) -> Option<&Avis<UntypedAst>> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.local_vars.get(key) {
                return  Some(v);
            }
        }

        None
    }

    fn find_ssa(&self, key: Key) -> Option<&Expr<UntypedAst>> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.local_ssa.get(key) {
                return  Some(v);
            }
        }

        None
    }

    fn push_scope(&mut self, fundef: Block<UntypedAst>) {
        self.scopes.push(fundef);
    }

    fn pop_scope(&mut self) -> Block<UntypedAst> {
        self.scopes.pop().unwrap()
    }
}

impl Rewriter for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Err = InferenceError;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err> {
        self.args = Vec::new();
        for arg in fundef.args {
            let ty = arg.ty.clone().expect("function argument cannot be untyped");
            self.args.push(Avis::new(arg.key, &arg.name, ty));
        }

        let block = self.trav_block(fundef.block)?;

        let mut args = Vec::new();
        mem::swap(&mut args, &mut self.args);
        Ok(Fundef {
            name: fundef.name,
            args,
            block,
        })
    }

    fn trav_block(&mut self, block: Block<Self::InAst>) -> Result<Block<Self::OutAst>, Self::Err> {
        let ret = self.trav_ssa(block.ret)?;

        let mut vars = Arena::new();
        mem::swap(&mut self.new_vars, &mut vars);
        let mut ssa = SecondaryArena::new();
        mem::swap(&mut self.new_ssa, &mut ssa);

        Ok(Block {
            local_vars: vars,
            local_ssa: ssa,
            ret,
        })
    }

    fn trav_ssa(&mut self, id: ArgOrVar) -> Result<ArgOrVar, Self::Err> {
        let id = match id {
            ArgOrVar::Arg(i) => {
                let ty = self.args[i].ty.clone();
                self.found_ty = Some(ty);
                ArgOrVar::Arg(i)
            },
            ArgOrVar::Var(old_key) => {
                let old_avis = self.find_id(old_key).unwrap().clone();
                let old_expr = self.find_ssa(old_key).unwrap().clone();
                let new_expr = self.trav_expr(old_expr)?;

                let new_key = self.new_vars.insert_with(|new_key| {
                    Avis { name: old_avis.name.to_owned(), ty: self.found_ty.clone().unwrap(), key: ArgOrVar::Var(new_key) }
                });
                println!("replaced {:?} by {:?} = {:?}", old_key, new_key, new_expr);
                self.new_ssa.insert(new_key, new_expr);
                ArgOrVar::Var(new_key)
            },
            ArgOrVar::Iv(old_key) => {
                ArgOrVar::Iv(old_key)
            },
        };

        Ok(id)
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<Tensor<Self::OutAst>, Self::Err> {
        let iv = self.trav_iv(tensor.iv)?;
        let lb = self.trav_ssa(tensor.lb)?;
        let ub = self.trav_ssa(tensor.ub)?;

        let body = self.trav_block(tensor.body)?;
        let ety = self.found_ty.take().unwrap();

        self.found_ty = Some(Type { basetype: ety.basetype, shp: Shape::Vector((if let Shape::Scalar = ety.shp { "." } else { "*" }).to_owned()) });
        Ok(Tensor { iv, body, lb, ub })
    }

    fn trav_iv(&mut self, iv: IndexVector) -> Result<IndexVector, Self::Err> {
        let old_avis = self.find_id(iv.0).unwrap().clone();
        let new_key = self.new_vars.insert_with(|new_key| {
            Avis { name: old_avis.name.to_owned(), ty: Type { basetype: BaseType::U32, shp: Shape::Scalar }, key: ArgOrVar::Iv(new_key) }
        });
        println!("replaced index vector {:?} by {:?}", iv.0, new_key);
        Ok(IndexVector(new_key))
    }

    fn trav_binary(&mut self, binary: Binary) -> Result<Binary, Self::Err> {
        let l = self.trav_ssa(binary.l)?;
        let _lty = self.found_ty.take().unwrap();
        let r = self.trav_ssa(binary.r)?;
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

    fn trav_unary(&mut self, unary: Unary) -> Result<Unary, Self::Err> {
        let r = self.trav_ssa(unary.r)?;
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

    fn trav_bool(&mut self, value: bool) -> Result<bool, Self::Err> {
        self.found_ty = Some(Type { basetype: BaseType::Bool, shp: Shape::Scalar });
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32) -> Result<u32, Self::Err> {
        self.found_ty = Some(Type { basetype: BaseType::U32, shp: Shape::Scalar });
        Ok(value)
    }
}
