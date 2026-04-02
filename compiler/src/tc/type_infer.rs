use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    Ok(TypeInfer::new().trav_program(program))
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Farg>,
    scopes: Vec<ScopeBlock<'ast, UntypedAst>>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_ids: Vec<&'ast VarInfo<'ast, TypedAst>>,
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

    fn alloc_farg(&self, name: String, ty: Type) -> &'ast Farg {
        Box::leak(Box::new(Farg { name, ty }))
    }

    fn alloc_lvis(&self, name: String, ty: Type, ssa: Option<&'ast Expr<'ast, TypedAst>>) -> &'ast VarInfo<'ast, TypedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        Box::leak(Box::new(expr))
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    ///
    /// Declarations
    ///

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        self.args = fundef.args.clone();
        let fundef_scope = fundef.scope_block();
        self.scopes.push(fundef_scope);
        self.new_ssa.push(Vec::new());
        self.idmap.clear();
        self.new_ids.clear();

        let (ret, _) = self.trav_id(fundef.ret_id());

        let mut new_args = Vec::with_capacity(fundef.args.len());
        for arg in fundef.args {
            new_args.push(self.trav_farg(arg));
        }

        self.scopes.pop().unwrap();
        let mut body = self
            .new_ssa
            .pop()
            .unwrap()
            .into_iter()
            .filter_map(|entry| match entry {
                ScopeEntry::Assign { lvis, expr } => Some(Stmt::Assign(Assign { lvis, expr })),
                ScopeEntry::IndexRange { .. } => None,
            })
            .collect::<Vec<_>>();
        body.push(Stmt::Return(Return { id: ret }));

        Fundef {
            name: fundef.name,
            ret_type: fundef.ret_type,
            args: new_args,
            decs: self.new_ids.clone(),
            body,
        }
    }

    fn trav_farg(&mut self, arg: &'ast Farg) -> &'ast Farg {
        self.alloc_farg(arg.name.clone(), arg.ty.clone())
    }

    fn trav_vardec(&mut self, _: &'ast VarInfo<'ast, Self::InAst>) -> &'ast VarInfo<'ast, Self::OutAst> {
        unreachable!("Vardecs should be replaced manually")
    }

    ///
    /// Statements
    ///

    fn trav_stmt(&mut self, stmt: Stmt<'ast, Self::InAst>) -> Stmt<'ast, Self::OutAst> {
        match stmt {
            Stmt::Assign(assign) => Stmt::Assign(self.trav_assign(assign)),
            Stmt::Return(ret) => Stmt::Return(self.trav_return(ret)),
        }
    }

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let before_len = self.new_ssa.last().map_or(0, |ssa| ssa.len());
        let _ = self.trav_id(Id::Var(assign.lvis));
        let ssa = self.new_ssa.last().expect("missing output scope in type inference");
        assert!(ssa.len() > before_len, "statement conversion did not emit output");
        let last = (*ssa.last().expect("missing emitted statement")).clone();
        match last {
            ScopeEntry::Assign { lvis, expr } => Assign { lvis, expr },
            ScopeEntry::IndexRange { .. } => panic!("expected assignment scope entry"),
        }
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        let (id, _) = self.trav_id(ret.id);
        Return { id }
    }

    ///
    /// Expressions
    ///

    type ExprOut = (Expr<'ast, Self::OutAst>, Type);

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        match expr {
            Tensor(n) => {
                let (expr, ty) = self.trav_tensor(n);
                (Tensor(expr), ty)
            },
            Binary(n) => {
                let (expr, ty) = self.trav_binary(n);
                (Binary(expr), ty)
            },
            Unary(n) => {
                let (expr, ty) = self.trav_unary(n);
                (Unary(expr), ty)
            },
            Id(n) => {
                let (id, ty) = self.trav_id(n);
                (Id(id), ty)
            },
            Bool(n) => {
                let (expr, ty) = self.trav_bool(n);
                (Bool(expr), ty)
            },
            U32(n) => {
                let (expr, ty) = self.trav_u32(n);
                (U32(expr), ty)
            },
        }
    }

    type TensorOut = (Tensor<'ast, Self::OutAst>, Type);

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        // Bounds are evaluated in the enclosing scope, not in the tensor body scope.
        let (lb, _check_if_vec) = self.trav_id(tensor.lb);
        let (ub, _check_if_vec) = self.trav_id(tensor.ub);

        self.scopes.last_mut().unwrap().push(
            ScopeEntry::IndexRange {
                iv: tensor.iv,
                lb: tensor.lb.clone().into(),
                ub: tensor.ub.clone().into(),
            });
        self.scopes.push(tensor.build_scope());
        self.new_ssa.push(Vec::new());

        let iv_new = self.alloc_lvis(tensor.iv.name.clone(), Type::scalar(BaseType::U32), None);
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);
        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
            iv: iv_new,
            lb: lb.clone(),
            ub: ub.clone(),
        });

        let (ret, _ret_ty) = self.trav_id(tensor.ret);

        self.scopes.pop().unwrap();
        let body = self
            .new_ssa
            .pop()
            .unwrap()
            .into_iter()
            .filter_map(|entry| match entry {
                ScopeEntry::Assign { lvis, expr } => Some(Stmt::Assign(Assign { lvis, expr })),
                ScopeEntry::IndexRange { .. } => None,
            })
            .collect::<Vec<_>>();

        let tensor = Tensor { iv: iv_new, lb, ub, ret, body };
        (tensor, Type::vector(BaseType::U32, "."))
    }

    type BinaryOut = (Binary<'ast, Self::OutAst>, Type);

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut {
        let (l, l_ty) = self.trav_id(binary.l);
        let (r, r_ty) = self.trav_id(binary.r);
        let ty = unifies(l_ty, r_ty).unwrap();
        (Binary { l, r, op: binary.op }, ty)
    }

    type UnaryOut = (Unary<'ast, Self::OutAst>, Type);

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut {
        let (r, r_ty) = self.trav_id(unary.r);
        (Unary { r, op: unary.op }, r_ty)
    }

    ///
    /// Terminals
    ///

    type IdOut = (Id<'ast, Self::OutAst>, Type);

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        match id {
            Id::Arg(i) => {
                let ty = self.args[i].ty.clone();
                (Id::Arg(i), ty)
            },
            Id::Var(old) => {
                if let Some(new_id) = self.idmap.get(&(old as *const _)) {
                    let ty = new_id.ty.clone();
                    return (Id::Var(*new_id), ty);
                }

                match find_local_in_scopes(&self.scopes, old).unwrap() {
                    LocalDef::Assign(old_expr) => {
                        let (new_expr, new_ty) = self.trav_expr(old_expr.clone());

                        let expr_ref = self.alloc_expr(new_expr);
                        let new_id = self.alloc_lvis(old.name.clone(), new_ty.clone(), Some(expr_ref));
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);

                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::Assign {
                            lvis: new_id,
                            expr: expr_ref,
                        });

                        (Id::Var(new_id), new_ty)
                    }
                    LocalDef::IndexRange { lb, ub } => {
                        let (lb, _) = self.trav_id(lb);
                        let (ub, _) = self.trav_id(ub);

                        let new_id = self.alloc_lvis(old.name.clone(), Type::scalar(BaseType::U32), None);
                        self.idmap.insert(old as *const _, new_id);
                        self.new_ids.push(new_id);
                        self.new_ssa.last_mut().unwrap().push(ScopeEntry::IndexRange {
                            iv: new_id,
                            lb,
                            ub,
                        });

                        let ty = Type::scalar(BaseType::U32);
                        (Id::Var(new_id), ty)
                    }
                }
            }
        }
    }

    type BoolOut = (bool, Type);

    fn trav_bool(&mut self, value: bool) -> Self::BoolOut {
        (value, Type::scalar(BaseType::Bool))
    }

    type U32Out = (u32, Type);

    fn trav_u32(&mut self, value: u32) -> Self::U32Out {
        (value, Type::scalar(BaseType::U32))
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    Ok(a)
}
