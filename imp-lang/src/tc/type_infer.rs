use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    let mut typed_functions: HashMap<String, Fundef<'ast, TypedAst>> = HashMap::new();
    let impls = program.impls.clone();

    // Collect and sort function names to ensure deterministic ordering
    let mut func_names: Vec<_> = program.functions.keys().cloned().collect();
    func_names.sort();

    // First pass: create stub signatures for all functions to enable forward references
    let mut functions_by_name: HashMap<String, &'ast Fundef<'ast, TypedAst>> = HashMap::new();
    for name in &func_names {
        if let Some(fundef) = program.functions.get(name) {
            let stub = Box::leak(Box::new(Fundef {
                is_public: fundef.is_public,
                name: fundef.name.clone(),
                ret_type: fundef.ret_type.clone(),
                args: fundef.args.clone(),
                decs: Vec::new(),
                body: Vec::new(),
            }));
            functions_by_name.insert(name.clone(), stub);
        }
    }

    // Second pass: type-check each function with all function signatures available
    for name in func_names {
        let fundef = program.functions.get(&name).unwrap();
        let mut infer = TypeInfer::new(&functions_by_name, &impls);
        let typed = infer.trav_fundef(fundef.clone());
        if let Some(err) = infer.errors.into_iter().next() {
            return Err(err);
        }

        let typed_ref = Box::leak(Box::new(typed.clone()));
        functions_by_name.insert(name.clone(), typed_ref);
        typed_functions.insert(name.clone(), typed);
    }

    Ok(Program {
        functions: typed_functions,
        typesets: program.typesets,
        members: program.members,
        traits: program.traits,
        impls: program.impls,
    })
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_ids: Vec<&'ast VarInfo<'ast, TypedAst>>,
    errors: Vec<InferenceError>,
    /// Mapping of function names to typed signatures for direct-call resolution.
    functions: HashMap<String, &'ast Fundef<'ast, TypedAst>>,
    impls: Vec<ImplDef>,
}

#[allow(dead_code)]
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
        shape: ShapePattern,
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
    /// Call argument count mismatch.
    CallArgumentCountMismatch {
        func_name: String,
        expected: usize,
        provided: usize,
    },
    /// Call argument type mismatch.
    CallArgumentTypeMismatch {
        func_name: String,
        arg_index: usize,
        expected: Type,
        provided: Type,
    },
    PrimitiveArgumentCountMismatch {
        primitive: Prf,
        expected: usize,
        provided: usize,
    },
    PrimitiveArgumentKindMismatch {
        primitive: Prf,
        arg_index: usize,
        expected: &'static str,
        provided: Type,
    },
    TraitImplNotFound {
        trait_name: String,
        method_name: String,
        arg_types: Vec<Type>,
    },
}

impl<'ast> TypeInfer<'ast> {
    pub fn new(functions: &HashMap<String, &'ast Fundef<'ast, TypedAst>>, impls: &[ImplDef]) -> Self {
        Self {
            args: Vec::new(),
            idmap: HashMap::new(),
            new_ids: Vec::new(),
            errors: Vec::new(),
            functions: functions.clone(),
            impls: impls.to_vec(),
        }
    }

    fn has_trait_impl(&self, trait_name: &str, method_name: &str, arg_types: &[Type]) -> bool {
        let _ = method_name;
        self.impls.iter().any(|impl_def| {
            impl_def.trait_name == trait_name
                && impl_def.args.len() == arg_types.len()
                && impl_def.args.iter().zip(arg_types.iter()).all(|(poly, ty)| poly_matches_concrete(poly, ty))
        })
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

    fn shape_knowledge(shape: &ShapePattern) -> TypeKnowledge {
        match shape {
            ShapePattern::Scalar => TypeKnowledge::Scalar,
            ShapePattern::Any => TypeKnowledge::AUD,
            ShapePattern::Axes(axes) => {
                let has_rest = axes.iter().any(|a| matches!(a, AxisPattern::Rank(_)));
                if has_rest {
                    let min_rank = axes.iter().filter(|a| matches!(a, AxisPattern::Dim(_))).count() as u8;
                    TypeKnowledge::AUDGN { min_rank }
                } else if axes.iter().any(|a| matches!(a, AxisPattern::Dim(DimPattern::Any))) {
                    TypeKnowledge::AKD
                } else {
                    TypeKnowledge::AKS
                }
            }
        }
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
            ShapePattern::Scalar => ShapePattern::Axes(vec![leading]),
            ShapePattern::Axes(axes) => {
                let mut new_axes = Vec::with_capacity(1 + axes.len());
                new_axes.push(leading);
                new_axes.extend_from_slice(axes);
                ShapePattern::Axes(new_axes)
            }
            // Element shape is fully unknown; result shape is also fully unknown.
            ShapePattern::Any => ShapePattern::Any,
        };

        let knowledge = Self::shape_knowledge(&result_shape);
        Type { ty: base_ty, shape: result_shape, knowledge }
    }

    /// Given the type of `ub` (the upper-bound operand of a tensor expression),
    /// return:
    ///   - the type to assign to the index variable `iv`
    ///   - `Some(k)` as the number of leading `Any` dimensions the tensor adds to the
    ///     element type, or `None` when it cannot be determined statically (→ AUD result).
    ///
    /// Semantics:
    ///   - Scalar `ub`: backward-compatible 1-D iteration; `iv` gets the same base type.
    ///   - `ub : T[k]` (rank-1, known size k): k-dimensional iteration; `iv` is `usize[k]`.
    ///       - k = 0 → execute once (empty iteration space), result = element type.
    ///   - Anything else (unknown rank, `..rest` present, `Any` shape): unknown → AUD.
    fn tensor_iv_and_dims(ub_ty: &Type) -> (Type, Option<usize>) {
        match &ub_ty.shape {
            ShapePattern::Scalar => {
                // Backward-compatible 1-D: scalar ub, scalar iv of the same base type.
                (Type::scalar(ub_ty.ty.clone()), Some(1))
            }
            ShapePattern::Axes(axes)
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
                        // ub rank-1 but unknown length → k unknown.
                        let iv_ty = Type {
                            ty: ub_ty.ty.clone(),
                            shape: ShapePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                            knowledge: TypeKnowledge::AKD,
                        };
                        (iv_ty, None)
                    }
                    AxisPattern::Dim(DimPattern::Var(_)) => {
                        // Named extent (e.g., `usize[n]`): value unknown at compile time.
                        let iv_ty = Type {
                            ty: ub_ty.ty.clone(),
                            shape: ShapePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]),
                            knowledge: TypeKnowledge::AKD,
                        };
                        (iv_ty, None)
                    }
                    _ => unreachable!("guard ensured AxisPattern::Dim"),
                }
            }
            _ => {
                // Multi-rank ub, contains `..rest`, or `Any` shape: fully unknown.
                let iv_ty = Type {
                    ty: ub_ty.ty.clone(),
                    shape: ShapePattern::Any,
                    knowledge: TypeKnowledge::AUD,
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
            ShapePattern::Scalar => ShapePattern::Axes(leading_axes),
            ShapePattern::Axes(elem_axes) => {
                let mut new_axes = leading_axes;
                new_axes.extend(elem_axes);
                ShapePattern::Axes(new_axes)
            }
            ShapePattern::Any => {
                // Element shape fully unknown; result shape is also fully unknown.
                ShapePattern::Any
            }
        };
        let knowledge = Self::shape_knowledge(&result_shape);
        Type { ty: elem_ty.ty, shape: result_shape, knowledge }
    }

    /// Try to extract one named/constant `AxisPattern` per element from `ub`'s SSA-defining
    /// array literal.  Returns `None` if `ub` is not a locally-defined array literal.
    ///
    /// For `ub = [cols]` (arg) → `[Dim(Var("cols", Use))]`.
    /// For `ub = [3]`   (u32 literal absorbed into SSA) → `[Dim(Known(3))]`.
    /// For `ub = []`    → `[]` (execute-once case).
    fn extract_ub_axes(&self, ub: &Id<'ast, UntypedAst>) -> Option<Vec<AxisPattern>> {
        let lvis = match ub {
            Id::Var(v) => v,
            Id::Arg(_) => return None,
            Id::Dim(_) | Id::Shp(_) => return None,
            Id::DimAt(_, _) => return None,
        };

        let arr = match lvis.ssa? {
            Expr::Array(arr) => arr,
            _ => return None,
        };

        let mut axes = Vec::with_capacity(arr.elems.len());
        for elem in &arr.elems {
            let dp = match elem {
                Id::Arg(i) => DimPattern::Var(ExtentVar {
                    name: self.args[*i].name.clone(),
                    role: SymbolRole::Use,
                }),
                Id::Dim(i) => DimPattern::Var(ExtentVar {
                    name: format!("{}.dim", self.args[*i].name),
                    role: SymbolRole::Use,
                }),
                Id::Shp(_) => DimPattern::Any,
                Id::DimAt(i, k) => DimPattern::Var(ExtentVar {
                    name: format!("{}.shp[{k}]", self.args[*i].name),
                    role: SymbolRole::Use,
                }),
                Id::Var(v) => {
                    match v.ssa {
                        Some(Expr::Const(Const::Usize(val))) => DimPattern::Known(*val),
                        _ => DimPattern::Var(ExtentVar {
                            name: v.name.clone(),
                            role: SymbolRole::Use,
                        }),
                    }
                }
            };
            axes.push(AxisPattern::Dim(dp));
        }
        Some(axes)
    }

    fn expect_scalar_prf_arg(&mut self, primitive: Prf, arg_index: usize, ty: &Type) {
        if !ty.is_scalar() {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive,
                arg_index,
                expected: "scalar",
                provided: ty.clone(),
            });
        }
    }

    fn expect_bool_scalar_prf_arg(&mut self, primitive: Prf, arg_index: usize, ty: &Type) {
        if !(ty.is_scalar() && ty.ty == BaseType::Bool) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive,
                arg_index,
                expected: "bool scalar",
                provided: ty.clone(),
            });
        }
    }

    fn expect_usize_vector_prf_arg(&mut self, primitive: Prf, arg_index: usize, ty: &Type) {
        let is_vector = matches!(
            ty.shape,
            ShapePattern::Axes(ref axes)
                if axes.len() == 1 && matches!(axes[0], AxisPattern::Dim(_))
        );

        if !(is_vector && matches!(ty.ty, BaseType::Usize)) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive,
                arg_index,
                expected: "usize vector",
                provided: ty.clone(),
            });
        }
    }

    fn expect_array_prf_arg(&mut self, primitive: Prf, arg_index: usize, ty: &Type) {
        if !ty.is_array() {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive,
                arg_index,
                expected: "array",
                provided: ty.clone(),
            });
        }
    }

    fn prf_fallback_type(prf: Prf, arg_types: &[Type]) -> Type {
        match prf {
            Prf::LtSxS | Prf::LeSxS | Prf::GtSxS | Prf::GeSxS | Prf::EqSxS | Prf::NeSxS | Prf::NotS => {
                Type::scalar(BaseType::Bool)
            }
            Prf::SelVxA => {
                let base = arg_types[1].ty.clone();
                Type::scalar(base)
            }
            Prf::AddSxS |  Prf::SubSxS | Prf::MulSxS | Prf::DivSxS | Prf::NegS => {
                let base = arg_types[0].ty.clone();
                Type::scalar(base)
            }
        }
    }

    fn operator_trait(call_name: &str, arity: usize) -> Option<(&'static str, &'static str)> {
        match (call_name, arity) {
            ("+", 2) => Some(("Add", "+")),
            ("-", 2) => Some(("Sub", "-")),
            ("*", 2) => Some(("Mul", "*")),
            ("/", 2) => Some(("Div", "/")),
            ("sel", 2) => Some(("Sel", "sel")),
            ("==", 2) => Some(("Eq", "==")),
            ("!=", 2) => Some(("Ne", "!=")),
            ("<", 2) => Some(("Lt", "<")),
            ("<=", 2) => Some(("Le", "<=")),
            (">", 2) => Some(("Gt", ">")),
            (">=", 2) => Some(("Ge", ">=")),
            ("-", 1) => Some(("Neg", "-")),
            ("!", 1) => Some(("Not", "!")),
            _ => None,
        }
    }

    fn operator_fallback_type(call_name: &str, arg_types: &[Type]) -> Option<Type> {
        match call_name {
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "!" => Some(Type::scalar(BaseType::Bool)),
            "sel" => Some(Type::scalar(arg_types[1].ty.clone())),
            "+" | "-" | "*" | "/" => Some(arg_types[0].clone()),
            _ => None,
        }
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    // Declarations

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        let Fundef { is_public, name, ret_type, args, body, decs: _ } = fundef;

        self.args = args.clone();
        self.idmap.clear();
        self.new_ids.clear();

        let new_args: Vec<&'ast Farg> = args.into_iter().map(|arg| self.trav_farg(arg)).collect();

        let mut new_body = Vec::new();
        for stmt in body {
            new_body.push(self.trav_stmt(stmt));
        }

        Fundef {
            is_public,
            name,
            ret_type,
            args: new_args,
            decs: self.new_ids.clone(),
            body: new_body,
        }
    }

    fn trav_farg(&mut self, arg: &'ast Farg) -> &'ast Farg {
        self.alloc_farg(arg.name.clone(), arg.ty.clone())
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

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        let (id, _) = self.trav_id(ret.id);
        Return { id }
    }

    // Expressions

    type ExprOut = (Expr<'ast, Self::OutAst>, Type);

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        match expr {
            Call(n) => {
                let (call, ty) = self.trav_call(n);
                (Call(call), ty)
            }
            PrfCall(n) => {
                let (prf_call, ty) = self.trav_prf_call(n);
                (PrfCall(prf_call), ty)
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

    type CallOut = (Call<'ast, Self::OutAst>, Type);

    fn trav_call(&mut self, call: Call<'ast, Self::InAst>) -> Self::CallOut {
        let func_name = &call.id;

        if let Some((trait_name, method_name)) = Self::operator_trait(func_name, call.args.len()) {
            let mut typed_args = Vec::with_capacity(call.args.len());
            let mut arg_types = Vec::with_capacity(call.args.len());
            for arg in call.args {
                let (typed_arg, ty) = self.trav_id(arg);
                typed_args.push(typed_arg);
                arg_types.push(ty);
            }

            if !self.has_trait_impl(trait_name, method_name, &arg_types) {
                self.errors.push(InferenceError::TraitImplNotFound {
                    trait_name: trait_name.to_owned(),
                    method_name: method_name.to_owned(),
                    arg_types: arg_types.clone(),
                });
            }

            let ty = Self::operator_fallback_type(func_name, &arg_types)
                .unwrap_or_else(|| Type::scalar(BaseType::I32));
            let typed_call = Call {
                id: CallTarget::TraitMethod {
                    trait_name: trait_name.to_owned(),
                    method_name: method_name.to_owned(),
                },
                args: typed_args,
            };
            return (typed_call, ty);
        }

        let Some(&target) = self.functions.get(func_name) else {
            self.errors.push(InferenceError::UndefinedFunction { name: func_name.clone() });
            // Traverse arguments anyway to catch errors in them.
            for arg in call.args {
                let (_id, _ty) = self.trav_id(arg);
            }
            // Return a stub with a dummy error-type result. Use the first available function as a placeholder.
            let stub_target = self.functions.values().next().copied().expect("at least one function must be defined");
            let stub_call = Call { id: CallTarget::Function(stub_target), args: vec![] };
            return (stub_call, Type {
                ty: BaseType::I32,
                shape: ShapePattern::Any,
                knowledge: TypeKnowledge::AUD,
            });
        };

        // Check argument count.
        if call.args.len() != target.args.len() {
            self.errors.push(InferenceError::CallArgumentCountMismatch {
                func_name: func_name.clone(),
                expected: target.args.len(),
                provided: call.args.len(),
            });
        }

        // Type-check each argument.
        let mut typed_args = Vec::with_capacity(call.args.len());
        for (arg_idx, arg) in call.args.into_iter().enumerate() {
            let (typed_arg, arg_ty) = self.trav_id(arg);
            if arg_idx < target.args.len() {
                let expected_ty = &target.args[arg_idx].ty;
                if !types_compatible(expected_ty, &arg_ty) {
                    self.errors.push(InferenceError::CallArgumentTypeMismatch {
                        func_name: func_name.clone(),
                        arg_index: arg_idx,
                        expected: expected_ty.clone(),
                        provided: arg_ty,
                    });
                }
            }
            typed_args.push(typed_arg);
        }

        let typed_call = Call { id: CallTarget::Function(target), args: typed_args };
        (typed_call, target.ret_type.clone())
    }

    type TensorOut = (Tensor<'ast, Self::OutAst>, Type);

    type PrfCallOut = (PrfCall<'ast, Self::OutAst>, Type);

    fn trav_prf_call(&mut self, prf_call: PrfCall<'ast, Self::InAst>) -> Self::PrfCallOut {
        let primitive = prf_call.id;
        let expected_arity = match primitive {
            Prf::NegS | Prf::NotS => 1,
            _ => 2,
        };

        if prf_call.args.len() != expected_arity {
            self.errors.push(InferenceError::PrimitiveArgumentCountMismatch {
                primitive,
                expected: expected_arity,
                provided: prf_call.args.len(),
            });
        }

        let mut args = Vec::with_capacity(prf_call.args.len());
        let mut arg_types = Vec::with_capacity(prf_call.args.len());
        for arg in prf_call.args {
            let (typed_arg, ty) = self.trav_id(arg);
            args.push(typed_arg);
            arg_types.push(ty);
        }

        match primitive {
            Prf::AddSxS | Prf::SubSxS | Prf::MulSxS | Prf::DivSxS => {
                if let Some(lhs) = arg_types.first() {
                    self.expect_scalar_prf_arg(primitive, 0, lhs);
                }
                if let Some(rhs) = arg_types.get(1) {
                    self.expect_scalar_prf_arg(primitive, 1, rhs);
                }
                if let [lhs, rhs, ..] = arg_types.as_slice()
                    && lhs.ty != rhs.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive,
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: rhs.clone(),
                    });
                }
            }
            Prf::LtSxS | Prf::LeSxS | Prf::GtSxS | Prf::GeSxS | Prf::EqSxS | Prf::NeSxS => {
                if let Some(lhs) = arg_types.first() {
                    self.expect_scalar_prf_arg(primitive, 0, lhs);
                }
                if let Some(rhs) = arg_types.get(1) {
                    self.expect_scalar_prf_arg(primitive, 1, rhs);
                }
                if let [lhs, rhs, ..] = arg_types.as_slice()
                    && lhs.ty != rhs.ty {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive,
                        arg_index: 1,
                        expected: "same scalar type as lhs",
                        provided: rhs.clone(),
                    });
                }
            }
            Prf::NegS => {
                if let Some(arg) = arg_types.first() {
                    self.expect_scalar_prf_arg(primitive, 0, arg);
                }
            }
            Prf::NotS => {
                if let Some(arg) = arg_types.first() {
                    self.expect_bool_scalar_prf_arg(primitive, 0, arg);
                }
            }
            Prf::SelVxA => {
                self.expect_usize_vector_prf_arg(primitive, 0, &arg_types[0]);
                self.expect_array_prf_arg(primitive, 1, &arg_types[1]);
            }
        }

        let ty = Self::prf_fallback_type(primitive, &arg_types);
        (PrfCall { id: primitive, args }, ty)
    }

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        // Inspect ub's SSA expression *before* traversal so we can extract named extents.
        let ub_named_axes = self.extract_ub_axes(&tensor.ub);

        let (lb, _lb_ty) = self.trav_id(tensor.lb);
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

        let mut body = Vec::new();
        for stmt in tensor.body {
            body.push(self.trav_stmt(stmt));
        }

        let (ret, ret_ty) = self.trav_id(tensor.ret);

        let result_ty = match leading_axes {
            Some(axes) => Self::tensor_result_type(ret_ty, axes),
            None => Type {
                ty: ret_ty.ty,
                shape: ShapePattern::Any,
                knowledge: TypeKnowledge::AUD,
            },
        };

        let tensor = Tensor { iv: iv_new, lb, ub, ret, body };
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
            Id::Dim(i) => {
                (Id::Dim(i), Type::scalar(BaseType::Usize))
            }
            Id::Shp(i) => {
                let dim_name = match &self.args[i].ty.shape {
                    ShapePattern::Axes(axes) => axes.iter().find_map(|a| {
                        if let AxisPattern::Rank(cap) = a { Some(cap.dim_name.clone()) } else { None }
                    }),
                    _ => None,
                }.expect("Shp(i) must come from an arg with a RankCapture");
                (Id::Shp(i), Type::vector(BaseType::Usize, &dim_name))
            }
            Id::DimAt(i, k) => {
                (Id::DimAt(i, k), Type::scalar(BaseType::Usize))
            }
        }
    }

    type ConstOut = (Const, Type);

    fn trav_const(&mut self, c: Const) -> Self::ConstOut {
        use Const::*;
        match c {
            I32(v) => (I32(v), Type::scalar(BaseType::I32)),
            I64(v) => (I64(v), Type::scalar(BaseType::I64)),
            U32(v) => (U32(v), Type::scalar(BaseType::U32)),
            U64(v) => (U64(v), Type::scalar(BaseType::U64)),
            Usize(v) => (Usize(v), Type::scalar(BaseType::Usize)),
            F32(v) => (F32(v), Type::scalar(BaseType::F32)),
            F64(v) => (F64(v), Type::scalar(BaseType::F64)),
            Bool(v) => (Bool(v), Type::scalar(BaseType::Bool)),
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
fn shapes_compatible(expected: &ShapePattern, provided: &ShapePattern) -> bool {
    // A `d:shp` rank capture is AUD-compatible with any shape on either side.
    let has_rank = |axes: &[AxisPattern]| axes.iter().any(|a| matches!(a, AxisPattern::Rank(_)));
    if let ShapePattern::Axes(exp_axes) = expected
        && has_rank(exp_axes) { return true; }
    if let ShapePattern::Axes(prov_axes) = provided
        && has_rank(prov_axes) { return true; }
    match (expected, provided) {
        (ShapePattern::Scalar, ShapePattern::Scalar) => true,
        (ShapePattern::Any, _) | (_, ShapePattern::Any) => true,
        (ShapePattern::Axes(exp_axes), ShapePattern::Axes(prov_axes)) => {
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
        (DimPattern::Var(e), DimPattern::Var(p)) => e.name == p.name,
        _ => false,
    }
}

fn poly_matches_concrete(poly: &PolyType, ty: &Type) -> bool {
    let head_ok = match poly.head.as_str() {
        "i32" => ty.ty == BaseType::I32,
        "i64" => ty.ty == BaseType::I64,
        "u32" => ty.ty == BaseType::U32,
        "u64" => ty.ty == BaseType::U64,
        "usize" => ty.ty == BaseType::Usize,
        "f32" => ty.ty == BaseType::F32,
        "f64" => ty.ty == BaseType::F64,
        "bool" => ty.ty == BaseType::Bool,
        _ => true,
    };

    if !head_ok {
        return false;
    }

    match (&poly.shape, &ty.shape) {
        (None, ShapePattern::Scalar) => true,
        (None, _) => false,
        (Some(ShapePattern::Any), _) => true,
        (Some(ShapePattern::Scalar), ShapePattern::Scalar) => true,
        (Some(ShapePattern::Scalar), _) => false,
        (Some(ShapePattern::Axes(exp_axes)), ShapePattern::Axes(got_axes)) => {
            if exp_axes.iter().any(|a| matches!(a, AxisPattern::Rank(_))) {
                return true;
            }
            if exp_axes.len() != got_axes.len() {
                return false;
            }
            exp_axes.iter().zip(got_axes.iter()).all(|(e, g)| match (e, g) {
                (AxisPattern::Dim(DimPattern::Any), AxisPattern::Dim(_)) => true,
                (AxisPattern::Dim(DimPattern::Known(a)), AxisPattern::Dim(DimPattern::Known(b))) => a == b,
                (AxisPattern::Dim(DimPattern::Var(_)), AxisPattern::Dim(_)) => true,
                (AxisPattern::Rank(_), _) => true,
                _ => false,
            })
        }
        (Some(ShapePattern::Axes(exp_axes)), ShapePattern::Scalar) => exp_axes.is_empty(),
        (Some(ShapePattern::Axes(_)), ShapePattern::Any) => true,
    }
}
