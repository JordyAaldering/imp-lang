use std::collections::HashMap;

use crate::{ast::*, traverse::AstPass};

/// Type inference pass: transforms UntypedAst to TypedAst.
///
/// Walks the AST computing types for all expressions, variables, and function returns.
/// Creates new Avis entries for each SSA binding with properly inferred types.
pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    Ok(TypeInfer::new().pass_program(program))
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
    fn type_of(&self, id: ArgOrVar<'ast, TypedAst>) -> Type {
        match id {
            ArgOrVar::Arg(i) => self.args[i].ty.clone().unwrap(),
            ArgOrVar::Var(v) => v.ty.clone(),
        }
    }
}

impl<'ast> AstPass<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;
    type OutAst = TypedAst;

    fn pass_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        self.args = fundef.args.clone();
        let fundef_scope = fundef.scope_block();
        self.scopes.push(fundef_scope);
        self.new_ssa.push(Vec::new());
        self.idmap.clear();
        self.new_ids.clear();

        let ret = self.pass_id(fundef.ret_id());

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
            ids: self.new_ids.clone(),
            body,
        }
    }

    fn pass_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        match stmt {
            Stmt::Assign(assign) => Stmt::Assign(self.pass_assign(assign)),
            Stmt::Return(ret) => Stmt::Return(self.pass_return(ret)),
        }
    }

    fn pass_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let before_len = self.new_ssa.last().map_or(0, |ssa| ssa.len());
        let _ = self.pass_id(ArgOrVar::Var(assign.avis));
        let ssa = self.new_ssa.last().expect("missing output scope in type inference");
        assert!(ssa.len() > before_len, "statement conversion did not emit output");
        let last = *ssa.last().expect("missing emitted statement");
        match last {
            ScopeEntry::Assign { avis, expr } => Assign { avis, expr },
            ScopeEntry::IndexRange { .. } => panic!("expected assignment scope entry"),
        }
    }

    fn pass_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        Return { id: self.pass_id(ret.id) }
    }

    fn pass_scope_entry(&mut self, entry: ScopeEntry<'ast, Self::InAst>) -> ScopeEntry<'ast, Self::OutAst> {
        match entry {
            ScopeEntry::Assign { avis, expr } => {
                let assign = self.pass_assign(Assign { avis, expr });
                ScopeEntry::Assign {
                    avis: assign.avis,
                    expr: assign.expr,
                }
            }
            ScopeEntry::IndexRange { avis, .. } => {
                let before_len = self.new_ssa.last().map_or(0, |ssa| ssa.len());
                let _ = self.pass_id(ArgOrVar::Var(avis));
                let ssa = self.new_ssa.last().expect("missing output scope in type inference");
                assert!(ssa.len() > before_len, "scope entry conversion did not emit output");
                match *ssa.last().expect("missing emitted scope entry") {
                    ScopeEntry::IndexRange { avis, lb, ub } => ScopeEntry::IndexRange { avis, lb, ub },
                    ScopeEntry::Assign { .. } => panic!("expected index range scope entry"),
                }
            }
        }
    }

    fn pass_id(&mut self, id: ArgOrVar<'ast, Self::InAst>) -> ArgOrVar<'ast, Self::OutAst> {
        match id {
            ArgOrVar::Arg(i) => ArgOrVar::Arg(i),
            ArgOrVar::Var(old) => {
                if let Some(new_id) = self.idmap.get(&(old as *const _)) {
                    return ArgOrVar::Var(*new_id);
                }

                match self.find_local_def(old) {
                    LocalDef::Assign(old_expr) => {
                        let new_expr = self.pass_expr(old_expr.clone());
                        let new_ty = self.type_of_expr(&new_expr);

                        let new_id = self.alloc_avis(old.name.clone(), new_ty);
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);

                        let expr_ref = self.alloc_expr(new_expr);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Assign {
                            avis: new_id,
                            expr: expr_ref,
                        });

                        ArgOrVar::Var(new_id)
                    }
                    LocalDef::IndexRange { lb, ub } => {
                        let lb = self.pass_id(lb);
                        let ub = self.pass_id(ub);

                        let new_id = self.alloc_avis(old.name.clone(), Type::scalar(BaseType::U32));
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
                            avis: new_id,
                            lb,
                            ub,
                        });

                        ArgOrVar::Var(new_id)
                    }
                }
            }
        }
    }

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Expr<'ast, Self::OutAst> {
        use Expr::*;
        match expr {
            Tensor(n) => Tensor(self.pass_tensor(n)),
            Binary(n) => Binary(self.pass_binary(n)),
            Unary(n) => Unary(self.pass_unary(n)),
            Bool(n) => Bool(self.pass_bool(n)),
            U32(n) => U32(self.pass_u32(n)),
        }
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Tensor<'ast, Self::OutAst> {
        self.scopes.push(tensor.scope_block());
        self.new_ssa.push(Vec::new());

        let lb = self.pass_id(tensor.lb);
        let ub = self.pass_id(tensor.ub);

        let iv_new = self.alloc_avis(tensor.iv.name.clone(), Type::scalar(BaseType::U32));
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);
        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
            avis: iv_new,
            lb,
            ub,
        });

        let ret = self.pass_id(tensor.ret);

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

    fn pass_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Binary<'ast, Self::OutAst> {
        let l = self.pass_id(binary.l);
        let r = self.pass_id(binary.r);

        Binary { l, r, op: binary.op }
    }

    fn pass_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Unary<'ast, Self::OutAst> {
        let r = self.pass_id(unary.r);
        Unary { r, op: unary.op }
    }

    fn pass_bool(&mut self, value: bool) -> bool {
        value
    }

    fn pass_u32(&mut self, value: u32) -> u32 {
        value
    }
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
