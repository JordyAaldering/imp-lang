use crate::{arena::{Arena, SecondaryArena}, ast::*, traverse::Rewriter};

pub fn type_infer(program: Program<UntypedAst>) -> Result<Program<TypedAst>, InferenceError> {
    let mut fundefs = Vec::new();

    for fundef in program.fundefs {
        let (_, res) = TypeInfer::new().trav_fundef(fundef)?;
        fundefs.push(res);
    }

    Ok(Program { fundefs })
}

pub struct TypeInfer {
    new_ids: Vec<Arena<Avis<TypedAst>>>,
    new_ssa: Vec<SecondaryArena<Expr<TypedAst>>>,
    fargs: Vec<Avis<UntypedAst>>,
    scopes: Vec<(Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>)>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            new_ids: Vec::new(),
            new_ssa: Vec::new(),
            fargs: Vec::new(),
            scopes: Vec::new(),
        }
    }
}

impl Scoped<UntypedAst, TypedAst> for TypeInfer {
    fn fargs(&self) -> &Vec<Avis<UntypedAst>> {
        &self.fargs
    }

    fn set_fargs(&mut self, fargs: Vec<Avis<UntypedAst>>) {
        self.fargs = fargs
    }

    fn pop_fargs(&mut self) -> Vec<Avis<TypedAst>> {
        let mut args = Vec::new();
        for arg in &self.fargs {
            let ty = arg.ty.clone().unwrap();
            args.push(Avis::from(&arg, ty));
        }
        self.fargs.clear();
        args
    }

    fn scopes(&self) -> &Vec<(Arena<Avis<UntypedAst>>, SecondaryArena<Expr<UntypedAst>>)> {
        &self.scopes
    }

    fn push_scope(&mut self, ids: Arena<Avis<UntypedAst>>, ssa: SecondaryArena<Expr<UntypedAst>>) {
        self.scopes.push((ids, ssa));
        self.new_ids.push(Arena::new());
        self.new_ssa.push(SecondaryArena::new());
    }

    fn pop_scope(&mut self) -> (Arena<Avis<TypedAst>>, SecondaryArena<Expr<TypedAst>>) {
        self.scopes.pop().unwrap();
        let ids = self.new_ids.pop().unwrap();
        let ssa = self.new_ssa.pop().unwrap();
        (ids, ssa)
    }
}

impl Rewriter for TypeInfer {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    type Ok = Type;

    type Err = InferenceError;

    fn trav_fundef(&mut self, fundef: Fundef<Self::InAst>) -> Result<(Type, Fundef<Self::OutAst>), InferenceError> {
        self.set_fargs(fundef.args.clone());
        self.push_scope(fundef.ids.clone(), fundef.ssa.clone());

        let (ret_ty, ret) = self.trav_ssa(fundef.ret)?;

        let (ids, ssa) = self.pop_scope();
        let args = self.pop_fargs();

        Ok((ret_ty, Fundef {
            name: fundef.name,
            args,
            ids,
            ssa,
            ret,
        }))
    }

    fn trav_ssa(&mut self, id: ArgOrVar) -> Result<(Type, ArgOrVar), InferenceError> {
        let ty = match id {
            ArgOrVar::Arg(i) => {
                self.fargs[i].ty.clone().unwrap()
            },
            ArgOrVar::Var(key) => {
                let old_avis = self.find_key(key).cloned().unwrap();
                let old_expr = self.find_ssa(key).cloned().unwrap();
                let (new_ty, new_expr) = self.trav_expr(old_expr)?;

                let avis = Avis::from(&old_avis, new_ty.clone());
                self.new_ids.last_mut().unwrap().insert_with_key(key, avis);
                self.new_ssa.last_mut().unwrap().insert(key, new_expr);
                new_ty
            },
            ArgOrVar::Iv(_) => {
                Type::scalar(BaseType::U32)
            },
        };

        Ok((ty, id))
    }

    fn trav_tensor(&mut self, tensor: Tensor<Self::InAst>) -> Result<(Type, Tensor<Self::OutAst>), InferenceError> {
        self.push_scope(tensor.ids.clone(), tensor.ssa.clone());

        let (_, lb) = self.trav_ssa(tensor.lb)?;
        let (_, ub) = self.trav_ssa(tensor.ub)?;

        let old_avis = self.find_key(tensor.iv).cloned().unwrap();
        let avis = Avis::from(&old_avis, Type::scalar(BaseType::U32));
        self.new_ids.last_mut().unwrap().insert_with_key(tensor.iv, avis);

        let (ret_ty, ret) = self.trav_ssa(tensor.ret)?;

        let shp = if let Shape::Scalar = ret_ty.shp { "." } else { "*" };
        let tensor_ty = Type::vector(ret_ty.basetype, shp);

        let (ids, ssa) = self.pop_scope();

        Ok((tensor_ty, Tensor {
            iv: tensor.iv,
            lb,
            ub,
            ret,
            ids,
            ssa,
        }))
    }

    fn trav_binary(&mut self, Binary { l, r, op }: Binary) -> Result<(Type, Binary), Self::Err> {
        let (lty, l) = self.trav_ssa(l)?;
        let (rty, r) = self.trav_ssa(r)?;

        let ty = unifies(lty, rty)?;

        use Bop::*;
        let ty = match op {
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

        Ok((ty, Binary { l, r, op }))
    }

    fn trav_unary(&mut self, Unary { r, op }: Unary) -> Result<(Type, Unary), Self::Err> {
        let (rty, r) = self.trav_ssa(r)?;

        use Uop::*;
        let ty = match op {
            Neg => {
                // TODO: check if r_ty unifies with signed num
                Type { basetype: BaseType::U32, shp: rty.shp }
            },
            Not => {
                // TODO: check if r_ty unifies with bool
                Type { basetype: BaseType::Bool, shp: rty.shp }
            },
        };

        Ok((ty, Unary { r, op }))
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
