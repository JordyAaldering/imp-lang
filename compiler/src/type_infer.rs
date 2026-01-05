use crate::{arena::{Arena, SecondaryArena}, ast::*, traverse::Rewriter};

pub struct TypeInfer {
    typed_ids: Vec<Arena<Avis<TypedAst>>>,
    typed_ssa: Vec<SecondaryArena<Expr<TypedAst>>>,
    // todo: this should be included in the return type, probably we should
    // return Result<(Self::OK, Node), Self::Err> instead
    found_ty: Option<Type>,
    fargs: Vec<Avis<UntypedAst>>,
    scopes: Vec<Block<UntypedAst>>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            typed_ids: Vec::new(),
            typed_ssa: Vec::new(),
            found_ty: None,
            fargs: Vec::new(),
            scopes: Vec::new(),
        }
    }
}

impl Scoped<UntypedAst> for TypeInfer {
    fn fargs(&self) -> &Vec<Avis<UntypedAst>> {
        &self.fargs
    }

    fn fargs_mut(&mut self) -> &mut Vec<Avis<UntypedAst>> {
        &mut self.fargs
    }

    fn scopes(&self) -> &Vec<Block<UntypedAst>> {
        &self.scopes
    }

    fn scopes_mut(&mut self) -> &mut Vec<Block<UntypedAst>> {
        &mut self.scopes
    }
}

impl Rewriter for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Err = InferenceError;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<Fundef<Self::OutAst>, Self::Err> {
        self.fargs = fundef.args.clone();

        self.typed_ids.push(Arena::new());
        self.typed_ssa.push(SecondaryArena::new());
        let body = self.trav_block(fundef.body)?;

        let mut args = Vec::new();
        for arg in &self.fargs {
            let ty = arg.ty.clone().expect("function argument cannot be untyped");
            args.push(Avis::from(&arg, ty));
        }
        self.fargs.clear();

        Ok(Fundef {
            name: fundef.name,
            args,
            body,
        })
    }

    fn trav_block(&mut self, block: Block<Self::InAst>) -> Result<Block<Self::OutAst>, Self::Err> {
        let ret = self.trav_ssa(block.ret)?;

        let ids = self.typed_ids.pop().unwrap();
        let ssa = self.typed_ssa.pop().unwrap();
        Ok(Block {
            ids,
            ssa,
            ret,
        })
    }

    fn trav_ssa(&mut self, id: ArgOrVar) -> Result<ArgOrVar, Self::Err> {
        match id {
            ArgOrVar::Arg(i) => {
                let ty = self.fargs[i].ty.clone().expect("function argument cannot be untyped");
                self.found_ty = Some(ty);
            },
            ArgOrVar::Var(key) => {
                let old_avis = self.find_key(key).cloned().unwrap();
                let old_expr = self.find_ssa(key).cloned().unwrap();
                let new_expr = self.trav_expr(old_expr)?;

                let avis = Avis::from(&old_avis, self.found_ty.clone().unwrap());
                let depth = self.depth(key).unwrap();
                self.typed_ids[depth].insert_with_key(key, avis);
                self.typed_ssa[depth].insert(key, new_expr);
            },
            ArgOrVar::Iv(_) => {
                // Index vector in an expression position, get the type of the index vector
                self.found_ty = Some(Type::scalar(BaseType::U32));
            },
        };

        Ok(id)
    }

    fn trav_iv(&mut self, iv: IndexVector) -> Result<IndexVector, Self::Err> {
        let old_avis = self.find_key(iv.0).cloned().unwrap();
        let avis = Avis::from(&old_avis, Type::scalar(BaseType::U32));
        self.typed_ids.last_mut().unwrap().insert_with_key(iv.0, avis);
        Ok(iv)
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<Tensor<Self::OutAst>, Self::Err> {
        self.typed_ids.push(Arena::new());
        self.typed_ssa.push(SecondaryArena::new());

        let iv = self.trav_iv(tensor.iv)?;
        let lb = self.trav_ssa(tensor.lb)?;
        let ub = self.trav_ssa(tensor.ub)?;

        let body = self.trav_block(tensor.body)?;
        let ety = self.found_ty.take().unwrap();

        let shp = if let Shape::Scalar = ety.shp { "." } else { "*" };
        self.found_ty = Some(Type::vector(ety.basetype, shp));
        Ok(Tensor { iv, body, lb, ub })
    }

    fn trav_binary(&mut self, Binary { l, r, op }: Binary) -> Result<Binary, Self::Err> {
        let l = self.trav_ssa(l)?;
        let lty = self.found_ty.take().unwrap();
        let r = self.trav_ssa(r)?;
        let rty = self.found_ty.take().unwrap();

        let ty = unifies(lty, rty)?;
        // TODO: check if lty and rty unify

        use Bop::*;
        self.found_ty = Some(match op {
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
        });

        Ok(Binary { l, r, op })
    }

    fn trav_unary(&mut self, Unary { r, op }: Unary) -> Result<Unary, Self::Err> {
        let r = self.trav_ssa(r)?;
        let rty = self.found_ty.take().unwrap();

        use Uop::*;
        self.found_ty = Some(match op {
            Neg => {
                // TODO: check if r_ty unifies with signed num
                Type { basetype: BaseType::U32, shp: rty.shp }
            },
            Not => {
                // TODO: check if r_ty unifies with bool
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
        });

        Ok(Unary { r, op })
    }

    fn trav_bool(&mut self, value: bool) -> Result<bool, Self::Err> {
        self.found_ty = Some(Type::scalar(BaseType::Bool));
        Ok(value)
    }

    fn trav_u32(&mut self, value: u32) -> Result<u32, Self::Err> {
        self.found_ty = Some(Type::scalar(BaseType::U32));
        Ok(value)
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    // TODO
    Ok(a)
}
