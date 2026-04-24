use std::{collections::HashMap, mem};
use typed_arena::Arena;

use crate::ast::*;

pub fn type_infer<'ast>(program: &mut Program<'ast, UntypedAst>) -> Result<(), InferenceError> {
    validate_overload_families(&program.overloads)?;

    let mut stubs: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>> = HashMap::new();

    for (name, overloads) in &program.overloads {
        let mut stub_groups = HashMap::new();
        for (sig, fundefs) in overloads {
            let mut stub_fundefs = Vec::new();
            for fundef in fundefs {
                stub_fundefs.push(DispatchStub {
                    args: fundef.args.clone(),
                    ret_type: fundef.ret_type.clone(),
                });
            }
            stub_groups.insert(sig.clone(), stub_fundefs);
        }
        stubs.insert(name.clone(), stub_groups);
    }

    for fundef in program.fundefs.iter_mut() {
        let mut tc = TypeInfer::new(stubs.clone());
        tc.trav_fundef(fundef);

        if let Some(err) = tc.errors.into_iter().next() {
            return Err(err);
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct DispatchStub {
    args: Vec<Farg>,
    ret_type: Type,
}

fn validate_overload_families(overloads: &HashMap<String, HashMap<BaseSignature, Vec<&Fundef<'_, UntypedAst>>>>) -> Result<(), InferenceError> {
    for (name, group) in overloads {
        for (sig, fundefs) in group {
            let (first, rest) = fundefs.split_first().unwrap();
            let expected_ret_ty = &first.ret_type.ty;
            for fundef in rest {
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
    decs: Arena<VarInfo<'ast, UntypedAst>>,
    exprs: Arena<Expr<'ast, UntypedAst>>,
    typed: HashMap<*const VarInfo<'ast, UntypedAst>, Type>,
    stubs: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>>,
    errors: Vec<InferenceError>,
}

#[allow(unused)]
#[derive(Debug)]
pub enum InferenceError {
    SelectionIndexNotVector { ty: Type },
    SelectionIndexNotInteger { ty: Type },
    SelectionRankTooSmall { needed: usize, known_min_rank: Option<usize>, shape: TypePattern },
    InhomogeneousArray { element: usize, expected: Type, found: Type },
    UndefinedFunction { name: String },
    NoMatchingOverload { name: String, arg_bases: BaseSignature },
    CallArgumentTypeMismatch { func_name: String, arg_index: usize, expected: Type, provided: Type },
    AmbiguousOverload { name: String, arg_bases: BaseSignature },
    PrimitiveArgumentKindMismatch { primitive: String, arg_index: usize, expected: &'static str, provided: Type },
    InconsistentOverloadReturnBase { name: String, arg_bases: BaseSignature, expected: BaseType, found: BaseType },
    FoldSelectionTypeMismatch { expected: Type, found: Type },
    FoldFunPlaceholderCountMismatch { found: usize },
    FoldFunctionTypeMismatch { expected: Type, found: Type },
    MissingTypeAnnotation { name: String },
}

impl<'ast> TypeInfer<'ast> {
    fn new(overloads: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>>) -> Self {
        Self {
            args: Vec::new(),
            decs: Arena::new(),
            exprs: Arena::new(),
            typed: HashMap::new(),
            stubs: overloads,
            errors: Vec::new(),
        }
    }

    fn array_literal_type(&mut self, elem_types: Vec<Type>) -> Type {
        let count = elem_types.len();
        let Some(first) = elem_types.first() else {
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
            TypePattern::Any => TypePattern::Any,
        };

        Type { ty: base_ty, shape: result_shape }
    }

    fn tensor_iv_and_dims(ub_ty: &Type) -> (Type, Option<usize>) {
        match &ub_ty.shape {
            TypePattern::Scalar => unreachable!("cannot iterate over scalar ub"),
            TypePattern::Axes(axes) if axes.len() == 1 && matches!(axes[0], AxisPattern::Dim(_)) => {
                match &axes[0] {
                    AxisPattern::Dim(DimPattern::Known(k)) => (Type::vector_dim(ub_ty.ty.clone(), DimPattern::Known(*k)), Some(*k)),
                    AxisPattern::Dim(DimPattern::Any) | AxisPattern::Dim(DimPattern::Var(_)) => (
                        Type { ty: ub_ty.ty.clone(), shape: TypePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)]) },
                        Some(1),
                    ),
                    _ => unreachable!(),
                }
            }
            _ => (Type { ty: ub_ty.ty.clone(), shape: TypePattern::Any }, None),
        }
    }

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
            TypePattern::Any => TypePattern::Any,
        };
        Type { ty: elem_ty.ty, shape: result_shape }
    }

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
                Id::Var(v) => match v.ssa {
                    Some(Expr::Const(Const::Usize(val))) => DimPattern::Known(*val),
                    _ => DimPattern::Var(v.name.clone()),
                },
            };
            axes.push(AxisPattern::Dim(dp));
        }
        Some(axes)
    }

    fn resolve_overload(&mut self, func_name: &str, arg_types: &[Type]) -> (&DispatchStub, bool) {
        let Some(group) = self.stubs.get(func_name) else {
            self.errors.push(InferenceError::UndefinedFunction { name: func_name.to_owned() });
            panic!("undefined function: {}", func_name);
        };

        let key = BaseSignature {
            base_types: arg_types.iter().map(|t| t.ty.clone()).collect(),
        };

        let Some(candidates) = group.get(&key) else {
            self.errors.push(InferenceError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload for function: {}", func_name);
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
                matches.push(target);
            }
        }

        if matches.is_empty() {
            self.errors.push(InferenceError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload for function: {}", func_name);
        }

        let best_matches = maximal_candidates(&matches);
        let needs_runtime_dispatch = best_matches.len() > 1;
        let runtime_dispatch_allowed = arg_types.iter().any(type_requires_runtime_dispatch);

        if needs_runtime_dispatch && !runtime_dispatch_allowed {
            self.errors.push(InferenceError::AmbiguousOverload {
                name: func_name.to_owned(),
                arg_bases: key,
            });
        }

        (best_matches[0], needs_runtime_dispatch)
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type Ast = UntypedAst;

    type ExprOut = Type;

    const EXPR_DEFAULT: Self::ExprOut = unreachable!();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, UntypedAst>) {
        debug_assert!(self.args.is_empty());
        debug_assert!(self.typed.is_empty());
        debug_assert!(self.decs.len() == 0);
        debug_assert!(self.exprs.len() == 0);

        self.args = fundef.args.clone();
        self.decs = mem::take(&mut fundef.decs);
        self.exprs = mem::take(&mut fundef.exprs);

        for assign in &mut fundef.shape_prelude {
            self.trav_assign(assign);
        }

        let _ret_ty = self.trav_body(&mut fundef.body);

        fundef.decs = mem::take(&mut self.decs);
        fundef.exprs = mem::take(&mut self.exprs);
        self.typed.clear();
        self.args.clear();
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, UntypedAst>) {
        let ty = self.trav_expr(&mut assign.expr);
        self.typed.insert(assign.lhs as *const _, ty.clone());

        unsafe {
            let ptr = assign.lhs as *const VarInfo<'ast, UntypedAst> as *mut VarInfo<'ast, UntypedAst>;
            (*ptr).ty = Some(ty);
        }
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, UntypedAst>) -> Self::ExprOut {
        let cond_ty = self.trav_id(&mut cond.cond);

        if !(cond_ty.is_scalar() && cond_ty.ty == BaseType::Bool) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: "cond".to_owned(),
                arg_index: 0,
                expected: "bool scalar",
                provided: cond_ty,
            });
        }

        let then_ty = self.trav_body(&mut cond.then_branch);
        let else_ty = self.trav_body(&mut cond.else_branch);

        if !types_compatible(&then_ty, &else_ty) || !types_compatible(&else_ty, &then_ty) {
            self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                primitive: "cond".to_owned(),
                arg_index: 2,
                expected: "same type as true-branch",
                provided: then_ty.clone(),
            });
        }

        then_ty
    }

    fn trav_call(&mut self, call: &mut Call<'ast, UntypedAst>) -> Self::ExprOut {
        let mut arg_types = Vec::with_capacity(call.args.len());
        for arg in &mut call.args {
            let ty = self.trav_id(arg);
            arg_types.push(ty);
        }

        let (target, runtime_dispatch) = self.resolve_overload(&call.id, &arg_types);
        let out_ty = if runtime_dispatch {
            Type { ty: target.ret_type.ty.clone(), shape: TypePattern::Any }
        } else {
            target.ret_type.clone()
        };

        out_ty
    }

    fn trav_prf(&mut self, prf: &mut Prf<'ast, UntypedAst>) -> Self::ExprOut {
        use Prf::*;
        match prf {
            ShapeA(arr) => {
                let arr_ty = self.trav_id(arr);
                if !arr_ty.is_array() {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: "shape".to_owned(),
                        arg_index: 0,
                        expected: "array",
                        provided: arr_ty,
                    });
                }
                Type::vector_dim(BaseType::Usize, DimPattern::Any)
            }
            DimA(arr) => {
                let arr_ty = self.trav_id(arr);
                if !arr_ty.is_array() {
                    self.errors.push(InferenceError::PrimitiveArgumentKindMismatch {
                        primitive: "dim".to_owned(),
                        arg_index: 0,
                        expected: "array",
                        provided: arr_ty,
                    });
                }
                Type::scalar(BaseType::Usize)
            }
            SelVxA(idx, arr) => {
                let _idx_ty = self.trav_id(idx);
                let arr_ty = self.trav_id(arr);
                Type::scalar(arr_ty.ty)
            }
            AddSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.ty)
            }
            SubSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.ty)
            }
            MulSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.ty)
            }
            DivSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.ty)
            }
            LtSxS(l, r) | LeSxS(l, r) | GtSxS(l, r) | GeSxS(l, r) | EqSxS(l, r) | NeSxS(l, r) => {
                let _l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(BaseType::Bool)
            }
            NegS(r) => {
                let r_ty = self.trav_id(r);
                Type::scalar(r_ty.ty)
            }
            NotS(r) => {
                let _r_ty = self.trav_id(r);
                Type::scalar(BaseType::Bool)
            }
        }
    }

    fn trav_tensor(&mut self, tensor: &mut Tensor<'ast, UntypedAst>) -> Self::ExprOut {
        let ub_named_axes = self.extract_ub_axes(&tensor.ub);

        if let Some(lb) = &mut tensor.lb {
            self.trav_id(lb);
        }

        let ub_ty = self.trav_id(&mut tensor.ub);

        let (iv_ty, leading_k) = Self::tensor_iv_and_dims(&ub_ty);

        let leading_axes: Option<Vec<AxisPattern>> = ub_named_axes.or_else(|| {
            leading_k.map(|k| (0..k).map(|_| AxisPattern::Dim(DimPattern::Any)).collect())
        });

        self.typed.insert(tensor.iv as *const _, iv_ty.clone());

        unsafe {
            let ptr = tensor.iv as *const VarInfo<'ast, UntypedAst> as *mut VarInfo<'ast, UntypedAst>;
            (*ptr).ty = Some(iv_ty);
        }

        let ret_ty = self.trav_body(&mut tensor.body);

        let result_ty = match leading_axes {
            Some(axes) => Self::tensor_result_type(ret_ty, axes),
            None => Type {
                ty: ret_ty.ty,
                shape: TypePattern::Any,
            },
        };

        result_ty
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, UntypedAst>) -> Self::ExprOut {
        let neutral_ty = self.trav_id(&mut fold.neutral);

        let selection_ty = self.trav_tensor(&mut fold.selection);

        if !types_compatible(&neutral_ty, &selection_ty) || !types_compatible(&selection_ty, &neutral_ty) {
            self.errors.push(InferenceError::FoldSelectionTypeMismatch {
                expected: neutral_ty.clone(),
                found: selection_ty.clone(),
            });
        }

        let ret_ty = match &mut fold.foldfun {
            FoldFun::Name(id) => {
                let arg_types = vec![neutral_ty.clone(), selection_ty.clone()];
                let (target, runtime_dispatch) = self.resolve_overload(&id, &arg_types);
                let out_ty = if runtime_dispatch {
                    Type { ty: target.ret_type.ty.clone(), shape: TypePattern::Any }
                } else {
                    target.ret_type.clone()
                };
                out_ty
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

        neutral_ty
    }

    fn trav_array(&mut self, array: &mut Array<'ast, UntypedAst>) -> Self::ExprOut {
        let mut elem_types = Vec::with_capacity(array.elems.len());
        for value in &mut array.elems {
            let ty = self.trav_id(value);
            elem_types.push(ty);
        }
        self.array_literal_type(elem_types)
    }

    fn trav_id(&mut self, id: &mut Id<'ast, UntypedAst>) -> Self::ExprOut {
        match *id {
            Id::Arg(i) => self.args[i].ty.clone(),
            Id::Var(id) => {
                self.typed
                    .get(&(id as *const _))
                    .expect("Id::Var referenced before its assignment was processed")
                    .clone()
            }
        }
    }

    fn trav_const(&mut self, c: &mut Const) -> Self::ExprOut {
        use Const::*;
        match c {
            Bool(_) => Type::scalar(BaseType::Bool),
            Usize(_) => Type::scalar(BaseType::Usize),
            U32(_) => Type::scalar(BaseType::U32),
            U64(_) => Type::scalar(BaseType::U64),
            I32(_) => Type::scalar(BaseType::I32),
            I64(_) => Type::scalar(BaseType::I64),
            F32(_) => Type::scalar(BaseType::F32),
            F64(_) => Type::scalar(BaseType::F64),
        }
    }
}

fn types_compatible(expected: &Type, provided: &Type) -> bool {
    expected.ty == provided.ty && shapes_compatible(&expected.shape, &provided.shape)
}

fn shapes_compatible(expected: &TypePattern, provided: &TypePattern) -> bool {
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
            exp_axes.iter().zip(prov_axes.iter()).all(|(e, p)| axes_compatible(e, p))
        }
        _ => false,
    }
}

fn axes_compatible(expected: &AxisPattern, provided: &AxisPattern) -> bool {
    match (expected, provided) {
        (AxisPattern::Dim(exp_d), AxisPattern::Dim(prov_d)) => dims_compatible(exp_d, prov_d),
        (AxisPattern::Rank(_), AxisPattern::Rank(_)) => true,
        _ => false,
    }
}

fn dims_compatible(expected: &DimPattern, provided: &DimPattern) -> bool {
    match (expected, provided) {
        (DimPattern::Any, _) | (_, DimPattern::Any) => true,
        (DimPattern::Known(e), DimPattern::Known(p)) => e == p,
        (DimPattern::Var(_), DimPattern::Known(_)) => true,
        (DimPattern::Known(_), DimPattern::Var(_)) => true,
        (DimPattern::Var(_), DimPattern::Var(_)) => true,
    }
}

fn maximal_candidates<'a>(candidates: &[&'a DispatchStub]) -> Vec<&'a DispatchStub> {
    let mut maximal = Vec::new();

    'outer: for a in candidates {
        for b in candidates {
            if std::ptr::eq(*a, *b) {
                continue;
            }
            if overload_more_specific(&b.args, &a.args) {
                continue 'outer;
            }
        }
        maximal.push(*a);
    }

    maximal
}

fn overload_more_specific(a: &[Farg], b: &[Farg]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut any_strict = false;
    for (a_arg, b_arg) in a.iter().zip(b.iter()) {
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
        (TypePattern::Scalar, TypePattern::Axes(axes)) => axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))),
        (TypePattern::Any, TypePattern::Any) => true,
        (TypePattern::Axes(_), TypePattern::Any) => true,
        (TypePattern::Axes(a_axes), TypePattern::Scalar) => a_axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))),
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
