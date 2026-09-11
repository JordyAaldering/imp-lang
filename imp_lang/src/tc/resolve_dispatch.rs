//! This module implements dispatch resolution for overloaded functions. It takes a program with
//! untyped AST and a scope with typed AST, and produces a program with typed AST where all function
//! calls are resolved to specific overloads based on the argument types.
//!
//! It is not always possible to statically decide which overload to call, namely with respect to
//! shapes, which are not always statically known. For these cases, we generate `wrapper functions'.
//! These wrapper functions are generated for each overload of the same base types, but with different
//! shapes. The wrapper function checks the shapes of the arguments at runtime and dispatches to the
//! correct overload.
//!
//! It is required that all overloads are non-ambiguous, i.e., for any given set of argument types,
//! there is at most one overload that `most compatible' with those types. Where `most compatible' is
//! to say that, when there are multiple overloads, such as `int` and int[d:shp] (where both may fit
//! if d is 0), the `int` case is more specific than the `int[*]` case.
//!
//! For a failing example, consider:
//!
//! ```imp
//! foo(x: int, y: int[*]) -> int
//!
//! foo(x: int[*], y: int) -> int
//! ```
//!
//! Here, if we call `foo(5, 5)`, both overloads are compatible, and we can define no clear ordering
//! between these overloads. In this case, the user should make the signatures more specific:
//!
//! ```imp
//! foo(x: int, y: int[*]) -> int
//!
//! foo(x: int[+], y: int) -> int
//! ```
//!
//! Now, `foo(5, 5)` can only refer to the first case, and any higher-dimensional
//! case can only match either one, but not both.

use std::collections::HashMap;

use crate::ast::*;

pub fn resolve_dispatch<'ast>(program: Program<'ast, UntypedAst>, scope: &'ast Scope<'ast, TypedAst>) -> Result<Program<'ast, TypedAst>, DispatchError> {
    let mut families: HashMap<String, HashMap<BaseSignature, Vec<FundefId<'ast, TypedAst>>>> = HashMap::new();
    let mut stubs: id_arena::Arena<Fundef<'ast, TypedAst>> = id_arena::Arena::new();
    let mut work_items: Vec<(FundefId<'ast, TypedAst>, FundefId<'ast, UntypedAst>)> = Vec::new();

    for (name, groups) in &program.overloads.families {
        let mut out_groups = HashMap::new();
        for (sig, fundef_ids) in groups {
            let mut out_ids = Vec::new();
            for &fundef_id in fundef_ids {
                let fundef = program.fundef(fundef_id);
                let id = stubs.alloc(Fundef {
                    name: fundef.name.clone(),
                    ret_type: fundef.ret_type.clone(),
                    args: fundef.args.clone(),
                    shape_prelude: Vec::new(),
                    shape_facts: fundef.shape_facts.clone(),
                    decs: Vec::new(),
                    body: Body {
                        stmts: Vec::new(),
                        ret: Id::Arg(usize::MAX),
                    },
                });
                out_ids.push(id);
                work_items.push((id, fundef_id));
            }
            out_groups.insert(sig.clone(), out_ids);
        }
        families.insert(name.clone(), out_groups);
    }

    for (id, src_id) in work_items {
        let src_fundef = program.fundef(src_id);
        let mut lower = DispatchResolver::new(scope, &stubs, families.clone());
        let lowered = lower.lower_fundef(src_fundef);
        if let Some(err) = lower.errors.into_iter().next() {
            return Err(err);
        }
        stubs[id] = lowered;
    }

    Ok(Program {
        overloads: OverloadFamilies { families },
        fundefs: stubs,
    })
}

#[allow(unused)]
#[derive(Debug)]
pub enum DispatchError {
    MissingTypeAnnotation { name: String },
    UndefinedFunction { name: String },
    NoMatchingOverload { name: String, arg_bases: BaseSignature },
    AmbiguousOverload { name: String, arg_bases: BaseSignature },
}

struct DispatchResolver<'ast, 'stubs> {
    scope: &'ast Scope<'ast, TypedAst>,
    stubs: &'stubs id_arena::Arena<Fundef<'ast, TypedAst>>,
    args: Vec<Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_decs: Vec<&'ast VarInfo<'ast, TypedAst>>,
    errors: Vec<DispatchError>,
    overloads: HashMap<String, HashMap<BaseSignature, Vec<FundefId<'ast, TypedAst>>>>,
}

impl<'ast, 'stubs> DispatchResolver<'ast, 'stubs> {
    fn new(
        scope: &'ast Scope<'ast, TypedAst>,
        stubs: &'stubs id_arena::Arena<Fundef<'ast, TypedAst>>,
        overloads: HashMap<String, HashMap<BaseSignature, Vec<FundefId<'ast, TypedAst>>>>,
    ) -> Self {
        Self {
            scope,
            stubs,
            args: Vec::new(),
            idmap: HashMap::new(),
            new_decs: Vec::new(),
            errors: Vec::new(),
            overloads,
        }
    }

    fn alloc_avis(&mut self, name: String, ty: Type, ssa: Option<&'ast ExprCell<'ast, TypedAst>>) -> &'ast VarInfo<'ast, TypedAst> {
        let var = self.scope.alloc_avis(name, ty, ssa);
        self.new_decs.push(var);
        var
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast ExprCell<'ast, TypedAst> {
        self.scope.alloc_expr(expr)
    }

    fn require_ty(&mut self, name: &str, ty: &Option<Type>) -> Type {
        match ty {
            Some(ty) => ty.clone(),
            None => {
                self.errors.push(DispatchError::MissingTypeAnnotation {
                    name: name.to_owned(),
                });
                Type::scalar(BaseType::I32)
            }
        }
    }

    fn id_type(&mut self, id: &Id<'ast, TypedAst>) -> Type {
        match id {
            Id::Arg(i) => self.args[*i].ty.clone(),
            Id::Var(v) => v.ty.clone(),
        }
    }

    fn resolve_target(&mut self, func_name: &str, arg_types: &[Type]) -> FundefId<'ast, TypedAst> {
        let Some(group) = self.overloads.get(func_name) else {
            self.errors.push(DispatchError::UndefinedFunction {
                name: func_name.to_string(),
            });
            panic!("undefined function during dispatch resolution: {}", func_name);
        };

        let key = BaseSignature {
            base_types: arg_types.iter().map(|ty| ty.basetype.clone()).collect(),
        };

        let Some(candidates) = group.get(&key) else {
            self.errors.push(DispatchError::NoMatchingOverload {
                name: func_name.to_string(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload during dispatch resolution: {}", func_name);
        };

        let mut matched = None;

        // At this point, candidates are already sorted by specificity,
        // so we can just take the first one that is compatible with the provided types.
        for id in candidates {
            for (candidate, provided) in self.stubs[*id].args.iter().zip(arg_types.iter()) {
                if true {
                    matched = Some(*id);
                    break;
                }
            }
        }

        let Some(matched) = matched else {
            self.errors.push(DispatchError::NoMatchingOverload {
                name: func_name.to_string(),
                arg_bases: key.clone(),
            });
            panic!("no compatible overload during dispatch resolution: {}", func_name);
        };

        matched
    }

    fn lower_fundef(&mut self, fundef: &Fundef<'ast, UntypedAst>) -> Fundef<'ast, TypedAst> {
        self.args = fundef.args.clone();
        self.idmap.clear();
        debug_assert!(self.new_decs.is_empty());

        let mut shape_prelude = Vec::new();
        for assign in &fundef.shape_prelude {
            shape_prelude.push(self.lower_assign(*assign));
        }

        let body = self.lower_body(fundef.body.clone());

        let decs = std::mem::take(&mut self.new_decs);

        Fundef {
            name: fundef.name.clone(),
            ret_type: fundef.ret_type.clone(),
            args: fundef.args.clone(),
            shape_prelude,
            shape_facts: fundef.shape_facts.clone(),
            decs,
            body,
        }
    }

    fn lower_body(&mut self, body: Body<'ast, UntypedAst>) -> Body<'ast, TypedAst> {
        let mut stmts = Vec::new();
        for stmt in body.stmts {
            stmts.push(self.lower_stmt(stmt));
        }
        let ret = self.lower_id(body.ret);
        Body { stmts, ret }
    }

    fn lower_stmt(&mut self, stmt: Stmt<'ast, UntypedAst>) -> Stmt<'ast, TypedAst> {
        match stmt {
            Stmt::Assign(a) => Stmt::Assign(self.lower_assign(a)),
            Stmt::Printf(p) => Stmt::Printf(self.lower_printf(p)),
        }
    }

    fn lower_assign(&mut self, assign: Assign<'ast, UntypedAst>) -> Assign<'ast, TypedAst> {
        let expr = self.lower_expr(assign.expr.borrow().clone());
        let expr_ref = self.alloc_expr(expr);
        let lhs_ty = self.require_ty(&assign.lhs.name, &assign.lhs.ty.borrow());
        let lhs = self.alloc_avis(assign.lhs.name.clone(), lhs_ty, Some(expr_ref));
        self.idmap.insert(assign.lhs as *const _, lhs);
        Assign { lhs, expr: expr_ref }
    }

    fn lower_printf(&mut self, printf: Printf<'ast, UntypedAst>) -> Printf<'ast, TypedAst> {
        Printf {
            id: self.lower_id(printf.id),
        }
    }

    fn lower_expr(&mut self, expr: Expr<'ast, UntypedAst>) -> Expr<'ast, TypedAst> {
        match expr {
            Expr::Cond(n) => Expr::Cond(self.lower_cond(n)),
            Expr::Call(n) => Expr::Call(self.lower_call(n)),
            Expr::Prf(n) => Expr::Prf(self.lower_prf(n)),
            Expr::Fold(n) => Expr::Fold(self.lower_fold(n)),
            Expr::Tensor(n) => Expr::Tensor(self.lower_tensor(n)),
            Expr::Array(n) => Expr::Array(self.lower_array(n)),
            Expr::Id(n) => Expr::Id(self.lower_id(n)),
            Expr::Const(n) => Expr::Const(n),
        }
    }

    fn lower_cond(&mut self, cond: Cond<'ast, UntypedAst>) -> Cond<'ast, TypedAst> {
        Cond {
            cond: self.lower_id(cond.cond),
            then_branch: self.lower_body(cond.then_branch),
            else_branch: self.lower_body(cond.else_branch),
        }
    }

    fn lower_call(&mut self, call: Call<'ast, UntypedAst>) -> Call<'ast, TypedAst> {
        let mut args = Vec::with_capacity(call.args.len());
        for arg in call.args {
            args.push(self.lower_id(arg));
        }
        let arg_types = args.iter().map(|arg| self.id_type(arg)).collect::<Vec<_>>();
        let target = self.resolve_target(&call.id, &arg_types);
        Call {
            id: target,
            args,
        }
    }

    fn lower_prf(&mut self, prf: Prf<'ast, UntypedAst>) -> Prf<'ast, TypedAst> {
        use Prf::*;
        match prf {
            ShapeA(a) => ShapeA(self.lower_id(a)),
            DimA(a) => DimA(self.lower_id(a)),
            SelVxA(i, a) => SelVxA(self.lower_id(i), self.lower_id(a)),
            AddSxS(l, r) => AddSxS(self.lower_id(l), self.lower_id(r)),
            SubSxS(l, r) => SubSxS(self.lower_id(l), self.lower_id(r)),
            MulSxS(l, r) => MulSxS(self.lower_id(l), self.lower_id(r)),
            DivSxS(l, r) => DivSxS(self.lower_id(l), self.lower_id(r)),
            LtSxS(l, r) => LtSxS(self.lower_id(l), self.lower_id(r)),
            LeSxS(l, r) => LeSxS(self.lower_id(l), self.lower_id(r)),
            GtSxS(l, r) => GtSxS(self.lower_id(l), self.lower_id(r)),
            GeSxS(l, r) => GeSxS(self.lower_id(l), self.lower_id(r)),
            EqSxS(l, r) => EqSxS(self.lower_id(l), self.lower_id(r)),
            NeSxS(l, r) => NeSxS(self.lower_id(l), self.lower_id(r)),
            NegS(v) => NegS(self.lower_id(v)),
            NotS(v) => NotS(self.lower_id(v)),
        }
    }

    fn lower_fold(&mut self, fold: Fold<'ast, UntypedAst>) -> Fold<'ast, TypedAst> {
        let neutral = self.lower_id(fold.neutral);
        let selection = self.lower_tensor(fold.selection);

        let foldfun = match fold.foldfun {
            FoldFun::Name(name) => {
                let arg_types = vec![self.id_type(&neutral), self.id_type(&selection.body.ret)];
                let target = self.resolve_target(&name, &arg_types);
                FoldFun::Name(target)
            }
            FoldFun::Apply { .. } => {
                unimplemented!("dispatch resolution for partial-application fold is not implemented")
            }
        };

        Fold {
            neutral,
            foldfun,
            selection,
        }
    }

    fn lower_tensor(&mut self, tensor: Tensor<'ast, UntypedAst>) -> Tensor<'ast, TypedAst> {
        let iv_ty = self.require_ty(&tensor.iv.name, &tensor.iv.ty.borrow());
        let iv = self.alloc_avis(tensor.iv.name.clone(), iv_ty, None);
        self.idmap.insert(tensor.iv as *const _, iv);

        Tensor {
            iv,
            lb: tensor.lb.map(|lb| self.lower_id(lb)),
            ub: self.lower_id(tensor.ub),
            body: self.lower_body(tensor.body),
        }
    }

    fn lower_array(&mut self, array: Array<'ast, UntypedAst>) -> Array<'ast, TypedAst> {
        Array {
            elems: array.elems.into_iter().map(|id| self.lower_id(id)).collect(),
        }
    }

    fn lower_id(&mut self, id: Id<'ast, UntypedAst>) -> Id<'ast, TypedAst> {
        match id {
            Id::Arg(i) => Id::Arg(i),
            Id::Var(v) => {
                let mapped = self
                    .idmap
                    .get(&(v as *const _))
                    .expect("Id::Var referenced before its assignment was lowered");
                Id::Var(*mapped)
            }
        }
    }
}
