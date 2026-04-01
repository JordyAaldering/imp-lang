use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

/// Type inference pass: transforms UntypedAst to TypedAst.
///
/// Walks the AST computing types for all expressions, variables, and function returns.
/// Creates new Avis entries for each SSA binding with properly inferred types.
pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    Ok(TypeInfer::new().trav_program(program))
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Avis<UntypedAst>>,
    scopes: Vec<ScopeBlock<'ast, UntypedAst>>,
    idmap: HashMap<*const Avis<UntypedAst>, &'ast Avis<TypedAst>>,
    new_ids: Vec<&'ast Avis<TypedAst>>,
    new_ssa: Vec<ScopeBlock<'ast, TypedAst>>,
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
        find_local_in_scopes(&self.scopes, key).expect("missing local definition in type inference")
    }

    /// Get the type of an SSA value (must be TypedAst after pass_id).
    fn type_of(&self, id: Id<'ast, TypedAst>) -> Type {
        match id {
            Id::Arg(i) => self.args[i].ty.clone().unwrap(),
            Id::Var(v) => v.ty.clone(),
        }
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;
    type OutAst = TypedAst;

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        self.args = fundef.args.clone();
        let fundef_scope = fundef.scope_block();
        self.scopes.push(fundef_scope);
        self.new_ssa.push(Vec::new());
        self.idmap.clear();
        self.new_ids.clear();

        let ret = self.trav_id(fundef.ret_id());

        let new_args = self.args.iter().map(|arg| {
            self.alloc_avis(arg.name.clone(), arg.ty.clone().unwrap())
        }).collect::<Vec<_>>();

        self.scopes.pop().unwrap();
        let mut body = self
            .new_ssa
            .pop()
            .unwrap()
            .into_iter()
            .filter_map(|entry| match entry {
                ScopeEntry::Assign { avis, expr } => Some(Stmt::Assign(Assign { avis, expr })),
                ScopeEntry::IndexRange { .. } => None,
            })
            .collect::<Vec<_>>();
        body.push(Stmt::Return(Return { id: ret }));

        Fundef {
            name: fundef.name,
            args: new_args,
            decls: self.new_ids.clone(),
            body,
        }
    }

    fn trav_farg(&mut self, arg: &'ast Avis<Self::InAst>) -> &'ast Avis<Self::OutAst> {
        self.alloc_avis(arg.name.clone(), arg.ty.clone().unwrap())
    }

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        match stmt {
            Stmt::Assign(assign) => Stmt::Assign(self.trav_assign(assign)),
            Stmt::Return(ret) => Stmt::Return(self.trav_return(ret)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let before_len = self.new_ssa.last().map_or(0, |ssa| ssa.len());
        let _ = self.trav_id(Id::Var(assign.avis));
        let ssa = self.new_ssa.last().expect("missing output scope in type inference");
        assert!(ssa.len() > before_len, "statement conversion did not emit output");
        let last = *ssa.last().expect("missing emitted statement");
        match last {
            ScopeEntry::Assign { avis, expr } => Assign { avis, expr },
            ScopeEntry::IndexRange { .. } => panic!("expected assignment scope entry"),
        }
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        Return { id: self.trav_id(ret.id) }
    }

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Id<'ast, Self::OutAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(old) => {
                if let Some(new_id) = self.idmap.get(&(old as *const _)) {
                    return Id::Var(*new_id);
                }

                match self.find_local_def(old) {
                    LocalDef::Assign(old_expr) => {
                        let new_expr = self.trav_expr(old_expr.clone());
                        let new_ty = self.type_of_expr(&new_expr);

                        let new_id = self.alloc_avis(old.name.clone(), new_ty);
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);

                        let expr_ref = self.alloc_expr(new_expr);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Assign {
                            avis: new_id,
                            expr: expr_ref,
                        });

                        Id::Var(new_id)
                    }
                    LocalDef::IndexRange { lb, ub } => {
                        let lb = self.trav_id(lb);
                        let ub = self.trav_id(ub);

                        let new_id = self.alloc_avis(old.name.clone(), Type::scalar(BaseType::U32));
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
                            iv: new_id,
                            lb,
                            ub,
                        });

                        Id::Var(new_id)
                    }
                }
            }
        }
    }

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Expr<'ast, Self::OutAst> {
        use Expr::*;
        match expr {
            Tensor(n) => Tensor(self.trav_tensor(n)),
            Binary(n) => Binary(self.trav_binary(n)),
            Unary(n) => Unary(self.trav_unary(n)),
            Bool(n) => Bool(self.trav_bool(n)),
            U32(n) => U32(self.trav_u32(n)),
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Tensor<'ast, Self::OutAst> {
        self.scopes.push(tensor.scope_block());
        self.new_ssa.push(Vec::new());

        let lb = self.trav_id(tensor.lb);
        let ub = self.trav_id(tensor.ub);

        let iv_new = self.alloc_avis(tensor.iv.name.clone(), Type::scalar(BaseType::U32));
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);
        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
            iv: iv_new,
            lb,
            ub,
        });

        let ret = self.trav_id(tensor.ret);

        self.scopes.pop().unwrap();
        let body = self
            .new_ssa
            .pop()
            .unwrap()
            .into_iter()
            .filter_map(|entry| match entry {
                ScopeEntry::Assign { avis, expr } => Some(Stmt::Assign(Assign { avis, expr })),
                ScopeEntry::IndexRange { .. } => None,
            })
            .collect::<Vec<_>>();

        Tensor { iv: iv_new, lb, ub, ret, body }
    }

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Binary<'ast, Self::OutAst> {
        let l = self.trav_id(binary.l);
        let r = self.trav_id(binary.r);

        Binary { l, r, op: binary.op }
    }

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Unary<'ast, Self::OutAst> {
        let r = self.trav_id(unary.r);
        Unary { r, op: unary.op }
    }

    fn trav_bool(&mut self, value: bool) -> bool {
        value
    }

    fn trav_u32(&mut self, value: u32) -> u32 {
        value
    }

    type StmtOut = Stmt<'ast, Self::OutAst>;

    type AssignOut = Assign<'ast, Self::OutAst>;

    type ReturnOut = Return<'ast, Self::OutAst>;

    type ExprOut = Expr<'ast, Self::OutAst>;

    type TensorOut = Tensor<'ast, Self::OutAst>;

    type BinaryOut = Binary<'ast, Self::OutAst>;

    type UnaryOut = Unary<'ast, Self::OutAst>;

    type IdOut = Id<'ast, Self::OutAst>;

    type BoolOut = bool;

    type U32Out = u32;
}

impl<'ast> TypeInfer<'ast> {
    /// Compute the type of a typed expression.
    fn type_of_expr(&self, expr: &Expr<'ast, TypedAst>) -> Type {
        match expr {
            Expr::Tensor(_) => {
                // Tensor type shape is determined at render time
                Type::vector(BaseType::U32, ".")
            }
            Expr::Binary(Binary { l, r, op }) => {
                let lty = self.type_of(*l);
                let rty = self.type_of(*r);
                let basety = unifies(lty, rty).unwrap();

                use Bop::*;
                match op {
                    Add | Sub | Mul | Div => Type { basetype: BaseType::U32, shp: basety.shp },
                    Eq | Ne => Type { basetype: BaseType::Bool, shp: basety.shp },
                    Lt | Le | Gt | Ge => Type { basetype: BaseType::Bool, shp: basety.shp },
                }
            }
            Expr::Unary(Unary { r, op }) => {
                let rty = self.type_of(*r);
                use Uop::*;
                match op {
                    Neg => Type { basetype: BaseType::U32, shp: rty.shp },
                    Not => Type { basetype: BaseType::Bool, shp: rty.shp },
                }
            }
            Expr::Bool(_) => Type::scalar(BaseType::Bool),
            Expr::U32(_) => Type::scalar(BaseType::U32),
        }
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    Ok(a)
}
