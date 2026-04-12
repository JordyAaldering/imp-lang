use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    validate_overload_families(&program.overloads)?;

    // First pass: create stub signatures for all functions to enable forward references
    let mut stubs: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>> = HashMap::new();

    for (name, overloads) in &program.overloads {
        let mut stub_groups = HashMap::new();
        for (sig, fundefs) in overloads {
            let mut stub_fundefs = Vec::new();
            for fundef in fundefs {
                let stub: &Fundef<'_, _> = Box::leak(Box::new(Fundef {
                    name: fundef.name.clone(),
                    ret_type: fundef.ret_type.clone(),
                    args: fundef.args.clone(),
                    shape_prelude: Vec::new(),
                    shape_facts: ShapeFacts::default(),
                    decs: Vec::new(),
                    body: Body { stmts: vec![], ret: Id::Arg(usize::MAX) },
                }));
                stub_fundefs.push(stub);
            }

            stub_groups.insert(sig.clone(), stub_fundefs);
        }

        stubs.insert(name.clone(), stub_groups);
    }

    // Second pass: type-check each function with all overload signatures available
    let mut overloads = HashMap::new();

    for (name, groups) in program.overloads {
        let mut new_groups = HashMap::new();
        for (sig, fundefs) in groups {
            let mut new_fundefs = Vec::new();
            for fundef in fundefs {
                let mut infer = TypeInfer::new(stubs.clone());
                let fundef = infer.trav_fundef(fundef.clone());

                if let Some(err) = infer.errors.into_iter().next() {
                    return Err(err);
                }

                new_fundefs.push(fundef);
            }

            new_groups.insert(sig, new_fundefs);
        }

        overloads.insert(name, new_groups);
    }

    Ok(Program { overloads })
}

fn validate_overload_families(overloads: &HashMap<String, HashMap<BaseSignature, Vec<Fundef<'_, UntypedAst>>>>) -> Result<(), InferenceError> {
    for (name, group) in overloads {
        for (sig, fundefs) in group {
            let (first, fundefs) = fundefs.split_first().unwrap();
            let expected_ret_ty = &first.ret_type.ty;
            for fundef in fundefs {
                if &fundef.ret_type.ty != expected_ret_ty {
                    return Err(InferenceError::InconsistentOverloadReturnBase {
                        name: name.clone(),
                        arg_bases: sig.clone(),
                        expected: expected_ret_ty.clone(),
                        found: fundef.ret_type.ty.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

pub struct TypeInfer<'ast> {
    args: Vec<Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_ids: Vec<&'ast VarInfo<'ast, TypedAst>>,
    errors: Vec<InferenceError>,
    stubs: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>>,
}

#[allow(unused)]
#[derive(Debug)]
pub enum InferenceError {
    SelectionIndexNotVector {
        ty: Type,
    },
    SelectionIndexNotInteger {
        ty: Type,
    },
    SelectionRankTooSmall {
        needed: usize,
        known_min_rank: Option<usize>,
        shape: TypePattern,
    },
    /// Array literal elements have different base types or ranks.
    InhomogeneousArray {
        element: usize,
        expected: Type,
        found: Type,
    },
    /// Call to undefined function.
    UndefinedFunction {
        name: String,
    },
    /// Function exists, but no overload matches argument base types and count.
    NoMatchingOverload {
        name: String,
        arg_bases: BaseSignature,
    },
    /// Call argument type mismatch.
    CallArgumentTypeMismatch {
        func_name: String,
        arg_index: usize,
        expected: Type,
        provided: Type,
    },
    AmbiguousOverload {
        name: String,
        arg_bases: BaseSignature,
    },
    PrimitiveArgumentKindMismatch {
        primitive: String,
        arg_index: usize,
        expected: &'static str,
        provided: Type,
    },
    InconsistentOverloadReturnBase {
        name: String,
        arg_bases: BaseSignature,
        expected: BaseType,
        found: BaseType,
    },
    FoldSelectionTypeMismatch {
        expected: Type,
        found: Type,
    },
    FoldFunPlaceholderCountMismatch {
        found: usize,
    },
    FoldFunctionTypeMismatch {
        expected: Type,
        found: Type,
    },
}

impl<'ast> TypeInfer<'ast> {
    fn new(overloads: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>>) -> Self {
        Self {
            args: Vec::new(),
            idmap: HashMap::new(),
            new_ids: Vec::new(),
            errors: Vec::new(),
            stubs: overloads,
        }
    }

    fn alloc_lvis(&self, name: String, ty: Type, ssa: Option<&'ast Expr<'ast, TypedAst>>) -> &'ast VarInfo<'ast, TypedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        Box::leak(Box::new(expr))
    }

    /// Build the type of an array literal. The element count becomes a leading `Known(n)` dimension
    /// prepended to the element type's shape. Errors on base-type or rank mismatch between elements.
    fn array_literal_type(&mut self, elem_types: Vec<Type>) -> Type {
        let count = elem_types.len();

        let Some(first) = elem_types.first() else {
            // Empty literal: default to i32[0].
            return Type::vector_dim(BaseType::I32, DimPattern::Known(0));
        };

        let base_ty = first.ty.clone();
        let elem_shape = first.shape.clone();
        let elem_rank = first.rank();

        for (i, ty) in elem_types.iter().enumerate().skip(1) {
            if ty.ty != base_ty || ty.rank() != elem_rank {
                self.errors.push(InferenceError::InhomogeneousArray {
                    element: i,
                    expected: first.clone(),
                    found: ty.clone(),
                });
            }
        }

        let leading = AxisPattern::Dim(DimPattern::Known(count));
        let result_shape = match &elem_shape {
            TypePattern::Scalar => TypePattern::Axes(vec![leading]),
            TypePattern::Axes(axes) => {
                let mut new_axes = Vec::with_capacity(1 + axes.len());
                new_axes.push(leading);
                new_axes.extend_from_slice(axes);
                TypePattern::Axes(new_axes)
            }
            // Element shape is fully unknown; result shape is also fully unknown.
            TypePattern::Any => TypePattern::Any,
        };

        Type { ty: base_ty, shape: result_shape }
    }

    fn tensor_iv_and_dims(ub_ty: &Type) -> (Type, Option<usize>) {
        match &ub_ty.shape {
            TypePattern::Scalar => {
                unreachable!("cannot iterate over scalar ub")
            }
            TypePattern::Axes(axes)
                if axes.len() == 1 && matches!(axes[0], AxisPattern::Dim(_)) =>
            {
                // Rank-1 ub: its element count gives the iteration dimensionality k.
                match &axes[0] {
                    AxisPattern::Dim(DimPattern::Known(k)) => {
                        // iv has the same element type as ub; shape is [k].
                        let iv_ty = Type::vector_dim(ub_ty.ty.clone(), DimPattern::Known(*k));
                        (iv_ty, Some(*k))
                    }
                    AxisPattern::Dim(DimPattern::Any) => {
                        // ub rank-1 with unknown extent still implies one loop dimension.
                        let iv_ty = Type {
                            ty: ub_ty.ty.clone(),
                            shape: TypePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                        };
                        (iv_ty, Some(1))
                    }
                    AxisPattern::Dim(DimPattern::Var(_)) => {
                        // Named extent (e.g., `usize[n]`): rank is still statically 1.
                        let iv_ty = Type {
                            ty: ub_ty.ty.clone(),
                            shape: TypePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                        };
                        (iv_ty, Some(1))
                    }
                    _ => unreachable!("guard ensured AxisPattern::Dim"),
                }
            }
            _ => {
                // Multi-rank ub, contains `..rest`, or `Any` shape: fully unknown.
                let iv_ty = Type {
                    ty: ub_ty.ty.clone(),
                    shape: TypePattern::Any,
                };
                (iv_ty, None)
            }
        }
    }

    /// Prepend `leading_axes` to `elem_ty`'s shape to produce the result type of a tensor.
    ///
    /// If `leading_axes` is empty the element type is returned unchanged (execute-once case).
    /// The axes may carry concrete `Var` names (from `extract_ub_axes`) or be `Any`.
    fn tensor_result_type(elem_ty: Type, leading_axes: Vec<AxisPattern>) -> Type {
        if leading_axes.is_empty() {
            return elem_ty;
        }
        let result_shape = match elem_ty.shape {
            TypePattern::Scalar => TypePattern::Axes(leading_axes),
            TypePattern::Axes(elem_axes) => {
                let mut new_axes = leading_axes;
                new_axes.extend(elem_axes);
                TypePattern::Axes(new_axes)
            }
            TypePattern::Any => {
                // Element shape fully unknown; result shape is also fully unknown.
                TypePattern::Any
            }
        };
        Type { ty: elem_ty.ty, shape: result_shape }
    }

    /// Try to extract one named/constant `AxisPattern` per element from `ub`'s SSA-defining
    /// array literal.  Returns `None` if `ub` is not a locally-defined array literal.
    ///
    /// For `ub = [cols]` (arg) → `[Dim(Var("cols"))]`.
    /// For `ub = [3]`   (u32 literal absorbed into SSA) → `[Dim(Known(3))]`.
    /// For `ub = []`    → `[]` (execute-once case).
    fn extract_ub_axes(&self, ub: &Id<'ast, UntypedAst>) -> Option<Vec<AxisPattern>> {
        let lvis = match ub {
            Id::Var(v) => v,
            Id::Arg(_) => return None,
        };

        let arr = match lvis.ssa? {
            Expr::Array(arr) => arr,
            _ => return None,
        };

        let mut axes = Vec::with_capacity(arr.elems.len());
        for elem in &arr.elems {
            let dp = match elem {
                Id::Arg(i) => DimPattern::Var(self.args[*i].id.clone()),
                Id::Var(v) => {
                    match v.ssa {
                        Some(Expr::Const(Const::Usize(val))) => DimPattern::Known(*val),
                        _ => DimPattern::Var(v.name.clone()),
                    }
                }
            };
            axes.push(AxisPattern::Dim(dp));
        }
        Some(axes)
    }

    fn expect_scalar_prf_arg(&mut self, prf_name: &str, arg_index: usize, ty: &Type) {
        if !ty.is_scalar() {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: prf_name.to_owned(),
                arg_index,
                expected: "scalar",
                provided: ty.clone(),
            });
        }
    }

    fn expect_bool_scalar_prf_arg(&mut self, prf_name: &str, arg_index: usize, ty: &Type) {
        if !(ty.is_scalar() && ty.ty == BaseType::Bool) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: prf_name.to_owned(),
                arg_index,
                expected: "bool scalar",
                provided: ty.clone(),
            });
        }
    }

    fn expect_usize_vector_prf_arg(&mut self, prf_name: &str, arg_index: usize, ty: &Type) {
        let is_vector = matches!(
            ty.shape,
            TypePattern::Axes(ref axes)
                if axes.len() == 1 && matches!(axes[0], AxisPattern::Dim(_))
        );

        if !(is_vector && matches!(ty.ty, BaseType::Usize)) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: prf_name.to_owned(),
                arg_index,
                expected: "usize vector",
                provided: ty.clone(),
            });
        }
    }

    fn expect_array_prf_arg(&mut self, prf_name: &str, arg_index: usize, ty: &Type) {
        if !ty.is_array() {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: prf_name.to_owned(),
                arg_index,
                expected: "array",
                provided: ty.clone(),
            });
        }
    }

    fn trav_fold_selection(&mut self, tensor: Tensor<'ast, UntypedAst>) -> (Tensor<'ast, TypedAst>, Type) {
        let lb = if let Some(lb) = tensor.lb {
            let (lb, _lb_ty) = self.trav_id(lb);
            Some(lb)
        } else {
            None
        };

        let (ub, ub_ty) = self.trav_id(tensor.ub);

        let (iv_ty, _leading_k) = Self::tensor_iv_and_dims(&ub_ty);
        let iv = self.alloc_lvis(tensor.iv.name.clone(), iv_ty, None);
        self.idmap.insert(tensor.iv as *const _, iv);
        self.new_ids.push(iv);

        let (body, ret_ty) = self.trav_body(tensor.body);
        (Tensor { body, iv, lb, ub }, ret_ty)
    }

    fn resolve_dispatch(&mut self, func_name: &str, arg_types: &[Type]) -> (&'ast Fundef<'ast, TypedAst>, bool) {
        let Some(group) = self.stubs.get(func_name) else {
            self.errors.push(InferenceError::UndefinedFunction {
                    name: func_name.to_owned(),
            });
            panic!("undefined function: {}", func_name);
        };

        let base_types = arg_types.iter().map(|t| t.ty.clone()).collect();
        let key = BaseSignature { base_types };

        let Some(candidates) = group.get(&key) else {
            self.errors.push(InferenceError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload for function: {} with arg bases {:?}", func_name, key);
        };

        let mut matches = Vec::new();
        for target in candidates {
            let mut is_match = true;
            for (expected, provided) in target.args.iter().zip(arg_types.iter()) {
                if !types_compatible(&expected.ty, provided) {
                    is_match = false;
                    break;
                }
            }

            if is_match {
                matches.push(*target);
            }
        }

        if matches.is_empty() {
            self.errors.push(InferenceError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload for function: {} with arg bases {:?}", func_name, key);
        }

        let best_matches = maximal_candidates(&matches);
        let needs_runtime_dispatch = best_matches.len() > 1;
        let runtime_dispatch_allowed = arg_types.iter().any(type_requires_runtime_dispatch);

        if needs_runtime_dispatch && !runtime_dispatch_allowed {
            self.errors.push(InferenceError::AmbiguousOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
        }

        (best_matches[0], needs_runtime_dispatch)
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    // Declarations

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        self.args = fundef.args.clone();

        self.idmap.clear();
        self.new_ids.clear();

        let mut new_shape_prelude = Vec::new();
        for assign in fundef.shape_prelude {
            new_shape_prelude.push(self.trav_assign(assign));
        }

        let (body, _ret_ty) = self.trav_body(fundef.body);

        Fundef {
            name: fundef.name,
            ret_type: fundef.ret_type,
            args: fundef.args,
            shape_prelude: new_shape_prelude,
            shape_facts: fundef.shape_facts,
            decs: self.new_ids.clone(),
            body,
        }
    }

    // Statements

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let (new_expr, new_ty) = self.trav_expr((*assign.expr).clone());
        let expr_ref = self.alloc_expr(new_expr);
        let new_lvis = self.alloc_lvis(assign.lhs.name.clone(), new_ty, Some(expr_ref));
        self.idmap.insert(assign.lhs as *const _, new_lvis);
        self.new_ids.push(new_lvis);
        Assign { lhs: new_lvis, expr: expr_ref }
    }

    type BodyOut = (Body<'ast, Self::OutAst>, Type);

    fn trav_body(&mut self, body: Body<'ast, Self::InAst>) -> Self::BodyOut {
        let mut stmts = Vec::new();
        for stmt in body.stmts {
            stmts.push(self.trav_stmt(stmt));
        }

        let (ret, ret_ty) = self.trav_id(body.ret);
        (Body { stmts, ret }, ret_ty)
    }

    // Expressions

    type ExprOut = (Expr<'ast, Self::OutAst>, Type);

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        match expr {
            Cond(n) => {
                let (cond, ty) = self.trav_cond(n);
                (Cond(cond), ty)
            }
            Call(n) => {
                let (call, ty) = self.trav_call(n);
                (Call(call), ty)
            }
            PrfCall(n) => {
                let (prf_call, ty) = self.trav_prf_call(n);
                (PrfCall(prf_call), ty)
            }
            Fold(n) => {
                let (fold, ty) = self.trav_fold(n);
                (Fold(fold), ty)
            }
            Tensor(n) => {
                let (expr, ty) = self.trav_tensor(n);
                (Tensor(expr), ty)
            }
            Array(n) => {
                let (expr, ty) = self.trav_array(n);
                (Array(expr), ty)
            }
            Id(n) => {
                let (id, ty) = self.trav_id(n);
                (Id(id), ty)
            }
            Const(n) => {
                let (id, ty) = self.trav_const(n);
                (Const(id), ty)
            }
        }
    }

    type CondOut = (Cond<'ast, Self::OutAst>, Type);

    fn trav_cond(&mut self, cond: Cond<'ast, Self::InAst>) -> Self::CondOut {
        let (cond_id, cond_ty) = self.trav_id(cond.cond);
        self.expect_bool_scalar_prf_arg("cond", 0, &cond_ty);

        let (then_id, then_ty) = self.trav_body(cond.then_branch);
        let (else_id, else_ty) = self.trav_body(cond.else_branch);

        if !types_compatible(&then_ty, &else_ty) || !types_compatible(&else_ty, &then_ty) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: "cond".to_owned(),
                arg_index: 2,
                expected: "same type as true-branch",
                provided: then_ty.clone(),
            });
        }

        (Cond { cond: cond_id, then_branch: then_id, else_branch: else_id }, then_ty)
    }

    type CallOut = (Call<'ast, Self::OutAst>, Type);

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut {
        let func_name = &call.id;

        let mut typed_args = Vec::with_capacity(call.args.len());
        let mut arg_types = Vec::with_capacity(call.args.len());
        for arg in call.args {
            let (typed_arg, ty) = self.trav_id(arg);
            typed_args.push(typed_arg);
            arg_types.push(ty);
        }

        let (target, runtime_dispatch) = self.resolve_dispatch(func_name, &arg_types);
        let call_ty = if runtime_dispatch {
            Type {
                ty: target.ret_type.ty.clone(),
                shape: TypePattern::Any,
            }
        } else {
            target.ret_type.clone()
        };
        let typed_call = Call {
            id: CallTarget::Function(target),
            args: typed_args,
        };
        (typed_call, call_ty)
    }

    type FoldOut = (Fold<'ast, Self::OutAst>, Type);

    fn trav_fold(&mut self, fold: Fold<'ast, Self::InAst>) -> Self::FoldOut {
        let (neutral, neutral_ty) = self.trav_id(fold.neutral);
        let (selection, selection_ty) = self.trav_fold_selection(fold.selection);

        if !types_compatible(&neutral_ty, &selection_ty) || !types_compatible(&selection_ty, &neutral_ty) {
            self.errors.push(InferenceError::FoldSelectionTypeMismatch {
                expected: neutral_ty.clone(),
                found: selection_ty.clone(),
            });
        }

        let (foldfun, ret_ty) = match fold.foldfun {
            FoldFun::Name(id) => {
                let arg_types = vec![neutral_ty.clone(), selection_ty.clone()];
                let (target, runtime_dispatch) = self.resolve_dispatch(&id, &arg_types);
                let out_ty = if runtime_dispatch {
                    Type { ty: target.ret_type.ty.clone(), shape: TypePattern::Any }
                } else {
                    target.ret_type.clone()
                };
                (FoldFun::Name(CallTarget::Function(target)), out_ty)
            }
            FoldFun::Apply { .. } => {
                unimplemented!("'partial application' fold not yet supported")
            }
        };

        if !types_compatible(&neutral_ty, &ret_ty) || !types_compatible(&ret_ty, &neutral_ty) {
            self.errors.push(InferenceError::FoldFunctionTypeMismatch {
                expected: neutral_ty.clone(),
                found: ret_ty,
            });
        }

        (Fold { neutral, foldfun, selection }, neutral_ty)
    }

    type TensorOut = (Tensor<'ast, Self::OutAst>, Type);

    type PrfCallOut = (PrfCall<'ast, Self::OutAst>, Type);

    fn trav_prf_call(&mut self, prf: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut {
        let prf_name = prf.nameof();

        use PrfCall::*;
        match prf {
            ShapeA(arr) => {
                let (arr, arr_ty) = self.trav_id(arr);
                self.expect_array_prf_arg(prf_name, 0, &arr_ty);
                (ShapeA(arr), Type::vector_dim(BaseType::Usize, DimPattern::Any))
            }
            DimA(arr) => {
                let (arr, arr_ty) = self.trav_id(arr);
                self.expect_array_prf_arg(prf_name, 0, &arr_ty);
                (DimA(arr), Type::scalar(BaseType::Usize))
            }
            SelVxA(idx, arr) => {
                let (idx, idx_ty) = self.trav_id(idx);
                self.expect_usize_vector_prf_arg(prf_name, 0, &idx_ty);
                let (arr, arr_ty) = self.trav_id(arr);
                self.expect_array_prf_arg(prf_name, 1, &arr_ty);
                (SelVxA(idx, arr), Type::scalar(arr_ty.ty))
            }
            AddSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (AddSxS(l, r), Type::scalar(l_ty.ty))
            }
            SubSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (SubSxS(l, r), Type::scalar(l_ty.ty))
            }
            MulSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (MulSxS(l, r), Type::scalar(l_ty.ty))
            }
            DivSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (DivSxS(l, r), Type::scalar(l_ty.ty))
            }
            LtSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            LeSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            GtSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            GeSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            EqSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            NeSxS(l, r) => {
                let (l, l_ty) = self.trav_id(l);
                self.expect_scalar_prf_arg(prf_name, 0, &l_ty);
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 1, &r_ty);

                if l_ty.ty != r_ty.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: prf_name.to_owned(),
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: r_ty.clone(),
                    });
                }

                (LtSxS(l, r), Type::scalar(BaseType::Bool))
            }
            NegS(r) => {
                let (r, r_ty) = self.trav_id(r);
                self.expect_scalar_prf_arg(prf_name, 0, &r_ty);
                (NegS(r), r_ty)
            }
            NotS(r) => {
                let (r, r_ty) = self.trav_id(r);
                self.expect_bool_scalar_prf_arg(prf_name, 0, &r_ty);
                (NotS(r), r_ty)
            }
        }
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        // Inspect ub's SSA expression *before* traversal so we can extract named extents.
        let ub_named_axes = self.extract_ub_axes(&tensor.ub);

        let lb = if let Some(lb) = tensor.lb {
            let (lb, _lb_ty) = self.trav_id(lb);
            Some(lb)
        } else {
            None
        };

        let (ub, ub_ty) = self.trav_id(tensor.ub);

        // Determine iv's type using the typed ub shape (gives the rank / iv length).
        let (iv_ty, leading_k) = Self::tensor_iv_and_dims(&ub_ty);

        // Prefer the named axes from the SSA inspection; fall back to anonymous `Any` axes.
        let leading_axes: Option<Vec<AxisPattern>> = ub_named_axes.or_else(|| {
            leading_k.map(|k| (0..k).map(|_| AxisPattern::Dim(DimPattern::Any)).collect())
        });

        let iv_new = self.alloc_lvis(tensor.iv.name.clone(), iv_ty, None);
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);

        let (body, ret_ty) = self.trav_body(tensor.body);

        let result_ty = match leading_axes {
            Some(axes) => Self::tensor_result_type(ret_ty, axes),
            None => Type {
                ty: ret_ty.ty,
                shape: TypePattern::Any,
            },
        };

        let tensor = Tensor { iv: iv_new, lb, ub, body };
        (tensor, result_ty)
    }

    type ArrayOut = (Array<'ast, Self::OutAst>, Type);

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut {
        let mut values = Vec::with_capacity(array.elems.len());
        let mut elem_types = Vec::with_capacity(array.elems.len());

        for value in array.elems {
            let (value, ty) = self.trav_id(value);
            elem_types.push(ty);
            values.push(value);
        }

        let ty = self.array_literal_type(elem_types);
        (Array { elems: values }, ty)
    }

    // Terminals
    type IdOut = (Id<'ast, Self::OutAst>, Type);

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        match id {
            Id::Arg(i) => {
                let ty = self.args[i].ty.clone();
                (Id::Arg(i), ty)
            },
            Id::Var(old) => {
                let new_id = self.idmap.get(&(old as *const _))
                    .expect("Id::Var referenced before its assignment was processed");
                let ty = new_id.ty.clone();
                (Id::Var(*new_id), ty)
            },
        }
    }

    type ConstOut = (Const, Type);

    fn trav_const(&mut self, c: Const) -> Self::ConstOut {
        use Const::*;
        match c {
            Bool(v) => (Bool(v), Type::scalar(BaseType::Bool)),
            I32(v) => (I32(v), Type::scalar(BaseType::I32)),
            I64(v) => (I64(v), Type::scalar(BaseType::I64)),
            U32(v) => (U32(v), Type::scalar(BaseType::U32)),
            U64(v) => (U64(v), Type::scalar(BaseType::U64)),
            Usize(v) => (Usize(v), Type::scalar(BaseType::Usize)),
            F32(v) => (F32(v), Type::scalar(BaseType::F32)),
            F64(v) => (F64(v), Type::scalar(BaseType::F64)),
        }
    }
}

/// Check if two types are compatible for parameter passing.
/// For now, we use structural equality (no variance or polymorphism).
fn types_compatible(expected: &Type, provided: &Type) -> bool {
    expected.ty == provided.ty
        && shapes_compatible(&expected.shape, &provided.shape)
}

/// Check if two shape patterns are compatible.
/// For now, we use structural equality; later this could support variance.
fn shapes_compatible(expected: &TypePattern, provided: &TypePattern) -> bool {
    // A `d:shp` rank capture is wildcard-like for array shapes, but does not match scalars.
    let has_rank = |axes: &[AxisPattern]| axes.iter().any(|a| matches!(a, AxisPattern::Rank(_)));
    match (expected, provided) {
        (TypePattern::Scalar, TypePattern::Scalar) => true,
        (TypePattern::Any, _) | (_, TypePattern::Any) => true,
        (TypePattern::Axes(exp_axes), TypePattern::Axes(prov_axes)) => {
            if has_rank(exp_axes) || has_rank(prov_axes) {
                return true;
            }
            if exp_axes.len() != prov_axes.len() {
                return false;
            }
            exp_axes
                .iter()
                .zip(prov_axes.iter())
                .all(|(e, p)| axes_compatible(e, p))
        }
        // Mismatched scalar/axes are not compatible
        _ => false,
    }
}

/// Check if two axis patterns are compatible.
fn axes_compatible(expected: &AxisPattern, provided: &AxisPattern) -> bool {
    match (expected, provided) {
        (AxisPattern::Dim(exp_d), AxisPattern::Dim(prov_d)) => dims_compatible(exp_d, prov_d),
        (AxisPattern::Rank(_), AxisPattern::Rank(_)) => true,
        _ => false,
    }
}

/// Check if two dimension patterns are compatible.
fn dims_compatible(expected: &DimPattern, provided: &DimPattern) -> bool {
    match (expected, provided) {
        (DimPattern::Any, _) | (_, DimPattern::Any) => true,
        (DimPattern::Known(e), DimPattern::Known(p)) => e == p,
        // Named extents in function signatures are symbolic constraints, so they
        // may match concrete or renamed dimensions at call sites.
        (DimPattern::Var(_), DimPattern::Known(_)) => true,
        (DimPattern::Known(_), DimPattern::Var(_)) => true,
        (DimPattern::Var(_), DimPattern::Var(_)) => true,
    }
}

fn maximal_candidates<'ast>(candidates: &[&'ast Fundef<'ast, TypedAst>]) -> Vec<&'ast Fundef<'ast, TypedAst>> {
    let mut maximal: Vec<&Fundef<'_, TypedAst>> = Vec::new();

    'outer: for a in candidates {
        for b in candidates {
            if std::ptr::eq(*a, *b) {
                continue;
            }
            if overload_more_specific(b, a) {
                continue 'outer;
            }
        }
        maximal.push(*a);
    }

    maximal
}

fn overload_more_specific(a: &Fundef<'_, TypedAst>, b: &Fundef<'_, TypedAst>) -> bool {
    if a.args.len() != b.args.len() {
        return false;
    }

    let mut any_strict = false;
    for (a_arg, b_arg) in a.args.iter().zip(b.args.iter()) {
        let rel = shape_relation(&a_arg.ty.shape, &b_arg.ty.shape);
        match rel {
            ShapeRel::More => any_strict = true,
            ShapeRel::Equal => {}
            ShapeRel::Less | ShapeRel::Incomparable => return false,
        }
    }

    any_strict
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeRel {
    More,
    Equal,
    Less,
    Incomparable,
}

fn shape_relation(a: &TypePattern, b: &TypePattern) -> ShapeRel {
    if shape_more_or_equal(a, b) {
        if shape_more_or_equal(b, a) {
            ShapeRel::Equal
        } else {
            ShapeRel::More
        }
    } else if shape_more_or_equal(b, a) {
        ShapeRel::Less
    } else {
        ShapeRel::Incomparable
    }
}

fn shape_more_or_equal(a: &TypePattern, b: &TypePattern) -> bool {
    match (a, b) {
        (TypePattern::Scalar, TypePattern::Scalar) => true,
        (TypePattern::Scalar, TypePattern::Any) => true,
        (TypePattern::Scalar, TypePattern::Axes(axes)) => {
            axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_)))
        }

        (TypePattern::Any, TypePattern::Any) => true,

        (TypePattern::Axes(_), TypePattern::Any) => true,
        (TypePattern::Axes(a_axes), TypePattern::Scalar) => {
            a_axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_)))
        }
        (TypePattern::Axes(a_axes), TypePattern::Axes(b_axes)) => axes_more_or_equal(a_axes, b_axes),

        _ => false,
    }
}

fn axes_more_or_equal(a: &[AxisPattern], b: &[AxisPattern]) -> bool {
    let a_has_rank = a.iter().any(|axis| matches!(axis, AxisPattern::Rank(_)));
    let b_has_rank = b.iter().any(|axis| matches!(axis, AxisPattern::Rank(_)));

    if b_has_rank {
        return true;
    }
    if a_has_rank {
        return false;
    }

    if a.len() != b.len() {
        return false;
    }

    a.iter().zip(b.iter()).all(|(ax, bx)| axis_more_or_equal(ax, bx))
}

fn axis_more_or_equal(a: &AxisPattern, b: &AxisPattern) -> bool {
    match (a, b) {
        (AxisPattern::Rank(_), AxisPattern::Rank(_)) => true,
        (AxisPattern::Rank(_), _) => false,
        (_, AxisPattern::Rank(_)) => true,
        (AxisPattern::Dim(ad), AxisPattern::Dim(bd)) => dim_more_or_equal(ad, bd),
    }
}

fn dim_more_or_equal(a: &DimPattern, b: &DimPattern) -> bool {
    match (a, b) {
        (DimPattern::Known(x), DimPattern::Known(y)) => x == y,
        (DimPattern::Known(_), DimPattern::Var(_)) => true,
        (DimPattern::Known(_), DimPattern::Any) => true,
        (DimPattern::Var(x), DimPattern::Var(y)) => x == y,
        (DimPattern::Var(_), DimPattern::Any) => true,
        (DimPattern::Any, DimPattern::Any) => true,
        _ => false,
    }
}

fn type_requires_runtime_dispatch(ty: &Type) -> bool {
    match &ty.shape {
        TypePattern::Any => true,
        TypePattern::Axes(axes) => axes.iter().any(axis_requires_runtime_dispatch),
        TypePattern::Scalar => false,
    }
}

fn axis_requires_runtime_dispatch(axis: &AxisPattern) -> bool {
    match axis {
        AxisPattern::Rank(_) => true,
        AxisPattern::Dim(dim) => matches!(dim, DimPattern::Any | DimPattern::Var(_)),
    }
}

