use std::collections::HashMap;

use crate::{Phase, ast::*};

pub fn type_infer<'ast>(program: &mut Program<'ast, UntypedAst>) -> Result<(), InferenceError> {
    let mut stubs: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>> = HashMap::new();

    for (name, overloads) in &program.overloads.families {
        let mut stub_groups = HashMap::new();
        for (sig, fundef_ids) in overloads {
            let mut stub_fundefs = Vec::new();
            for &fundef_id in fundef_ids {
                let fundef = program.fundef(fundef_id);
                stub_fundefs.push(DispatchStub {
                    args: fundef.args.clone(),
                    ret_type: fundef.ret_type.clone(),
                });
            }
            stub_groups.insert(sig.clone(), stub_fundefs);
        }
        stubs.insert(name.clone(), stub_groups);
    }

    for (_, fundef) in program.fundefs.iter_mut() {
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

pub struct TypeInfer {
    args: Vec<Farg>,
    stubs: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>>,
    errors: Vec<InferenceError>,
}

#[allow(unused)]
#[derive(Debug)]
pub enum InferenceError {
    SelectionIndexNotVector { ty: Type },
    SelectionIndexNotInteger { ty: Type },
    SelectionRankTooSmall { needed: usize, known_min_rank: Option<usize>, shape: AxisPattern },
    InhomogeneousArray { element: usize, expected: Type, found: Type },
    UndefinedFunction { name: String },
    NoMatchingOverload { name: String, arg_bases: BaseSignature },
    CallArgumentTypeMismatch { func_name: String, arg_index: usize, expected: Type, provided: Type },
    PrimitiveArgumentKindMismatch { primitive: String, arg_index: usize, expected: &'static str, provided: Type },
    FoldSelectionTypeMismatch { expected: Type, found: Type },
    FoldFunPlaceholderCountMismatch { found: usize },
    FoldFunctionTypeMismatch { expected: Type, found: Type },
    MissingTypeAnnotation { name: String },
}

impl TypeInfer {
    fn new(overloads: HashMap<String, HashMap<BaseSignature, Vec<DispatchStub>>>) -> Self {
        Self {
            args: Vec::new(),
            stubs: overloads,
            errors: Vec::new(),
        }
    }

    fn array_literal_type(&mut self, elem_types: Vec<Type>) -> Type {
        let count = elem_types.len();
        let Some(first) = elem_types.first() else {
            return Type::new(BaseType::I32, Vec::new());
        };

        let base_ty = first.basetype.clone();
        let elem_rank = first.rank();

        for (i, ty) in elem_types.iter().enumerate().skip(1) {
            if ty.basetype != base_ty || ty.rank() != elem_rank {
                self.errors.push(InferenceError::InhomogeneousArray {
                    element: i,
                    expected: first.clone(),
                    found: ty.clone(),
                });
            }
        }

        let leading = AxisPattern::FixedDim { len: count };
        let result_shape = if let Some(axes) = first.type_pattern() {
            // (Potentially) non-scalar
            let mut new_axes = Vec::with_capacity(1 + axes.len());
            new_axes.push(leading);
            new_axes.extend_from_slice(axes);
            TypePattern::new(new_axes)
        } else {
            // Scalar
            TypePattern::scalar()
        };

        Type { basetype: base_ty, shape: result_shape }
    }

    fn tensor_iv_and_dims(ub_ty: &Type) -> (Type, Option<usize>) {
        let basetype = ub_ty.basetype.clone();
        if let Some(axis) = ub_ty.get_vector() {
            match axis {
                AxisPattern::FixedDim { len } => {
                    let ty = Type { basetype, shape: TypePattern::new(vec![AxisPattern::FixedDim { len: *len }]) };
                    (ty, Some(*len))
                }
                AxisPattern::VarDim { len } => {
                    let ty = Type { basetype, shape: TypePattern::new(vec![AxisPattern::VarDim { len: len.clone() }]) };
                    (ty, None)
                }
                AxisPattern::FixedShape { dim: 1, .. } => {
                    let ty = Type { basetype, shape: TypePattern::new(vec![AxisPattern::VarDim { len: None }]) };
                    (ty, None)
                }
                _ => unreachable!(),
            }
        } else {
            // unreachable!("cannot iterate over scalar ub")
            let ty = Type { basetype, shape: TypePattern::new(vec![AxisPattern::VarDim { len: None }]) };
            (ty, None)
        }
    }

    fn tensor_result_type(elem_ty: Type, leading_axes: Vec<AxisPattern>) -> Type {
        if leading_axes.is_empty() {
            return elem_ty;
        }

        let shape = if let Some(axes) = elem_ty.type_pattern() {
            let mut new_axes = leading_axes;
            new_axes.extend(axes.clone());
            TypePattern::new(new_axes)
        } else {
            TypePattern::new(leading_axes)
        };

        Type { basetype: elem_ty.basetype, shape }
    }

    fn extract_ub_axes<'ast>(&self, ub: &Id<'ast, UntypedAst>) -> Option<Vec<AxisPattern>> {
        let var = match ub {
            Id::Var(v) => v,
            Id::Arg(_) => return None,
        };

        let elems: Vec<Id<'ast, UntypedAst>> = match &*var.ssa?.borrow() {
            Expr::Array(arr) => arr.elems.clone(),
            _ => return None,
        };

        let mut axes = Vec::with_capacity(elems.len());
        for elem in &elems {
            let dp = match elem {
                Id::Arg(i) => AxisPattern::VarDim { len: Some(self.args[*i].id.clone()) },
                Id::Var(v) => {
                    let known_usize = v.ssa.and_then(|cell| match &*cell.borrow() {
                        Expr::Const(Const::Usize(val)) => Some(*val),
                        _ => None,
                    });
                    match known_usize {
                        Some(len) => AxisPattern::FixedDim { len },
                        None => AxisPattern::VarDim { len: None },
                    }
                }
            };
            axes.push(dp);
        }
        Some(axes)
    }

    fn resolve_overload(&mut self, func_name: &str, arg_types: &[Type]) -> (&DispatchStub, bool) {
        let Some(group) = self.stubs.get(func_name) else {
            self.errors.push(InferenceError::UndefinedFunction { name: func_name.to_owned() });
            panic!("undefined function: {}", func_name);
        };

        let key = BaseSignature {
            base_types: arg_types.iter().map(|t| t.basetype.clone()).collect(),
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

        (best_matches[0], needs_runtime_dispatch)
    }
}

impl<'ast> Traverse<'ast> for TypeInfer {
	const PHASE: Phase = Phase::TI;

    type Ast = UntypedAst;

    type DeclOut = ();

    type ExprOut = Type;

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, UntypedAst>) {
        debug_assert!(self.args.is_empty());

        self.args = fundef.args.clone();

        for assign in &mut fundef.shape_prelude {
            self.trav_assign(assign);
        }

        let _ret_ty = self.trav_body(&mut fundef.body);

        self.args.clear();
    }

    fn trav_assign(&mut self, assign: &mut Assign<'ast, UntypedAst>) {
        let ty = self.trav_expr(assign.expr);
        *assign.lhs.ty.borrow_mut() = Some(ty);
    }

    fn trav_cond(&mut self, cond: &mut Cond<'ast, UntypedAst>) -> Self::ExprOut {
        let cond_ty = self.trav_id(&mut cond.cond);

        if !(cond_ty.is_definitely_scalar() && cond_ty.basetype == BaseType::Bool) {
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
            Type::new_aud(target.ret_type.basetype.clone())
        } else {
            target.ret_type.clone()
        };

        out_ty
    }

    fn trav_prf(&mut self, prf: &mut Prf<'ast, UntypedAst>) -> Self::ExprOut {
        use Prf::*;
        match prf {
            ShapeA(_) => {
                Type { basetype: BaseType::Usize, shape: TypePattern::new(vec![AxisPattern::VarDim { len: None }]) }
            }
            DimA(arr) => {
                let arr_ty = self.trav_id(arr);
                if arr_ty.is_definitely_scalar() {
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
                Type::scalar(arr_ty.basetype)
            }
            AddSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.basetype)
            }
            SubSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.basetype)
            }
            MulSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.basetype)
            }
            DivSxS(l, r) => {
                let l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(l_ty.basetype)
            }
            LtSxS(l, r) | LeSxS(l, r) | GtSxS(l, r) | GeSxS(l, r) | EqSxS(l, r) | NeSxS(l, r) => {
                let _l_ty = self.trav_id(l);
                let _r_ty = self.trav_id(r);
                Type::scalar(BaseType::Bool)
            }
            NegS(r) => {
                let r_ty = self.trav_id(r);
                Type::scalar(r_ty.basetype)
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
            leading_k.map(|k| (0..k).map(|_| AxisPattern::VarDim { len: None }).collect())
        });

        *tensor.iv.ty.borrow_mut() = Some(iv_ty);

        let ret_ty = self.trav_body(&mut tensor.body);

        let result_ty = match leading_axes {
            Some(axes) => Self::tensor_result_type(ret_ty, axes),
            None => Type::new_aud(ret_ty.basetype),
        };

        result_ty
    }

    fn trav_fold(&mut self, fold: &mut Fold<'ast, UntypedAst>) -> Self::ExprOut {
        let neutral_ty = self.trav_id(&mut fold.neutral);

        let _selection_ty = self.trav_tensor(&mut fold.selection);

        //if !types_compatible(&neutral_ty, &selection_ty) {
        //    self.errors.push(InferenceError::FoldSelectionTypeMismatch {
        //        expected: neutral_ty.clone(),
        //        found: selection_ty.clone(),
        //    });
        //}

        let ret_ty = match &mut fold.foldfun {
            FoldFun::Name(id) => {
                let arg_types = vec![neutral_ty.clone(), neutral_ty.clone()];
                let (target, runtime_dispatch) = self.resolve_overload(&id, &arg_types);
                let out_ty = if runtime_dispatch {
                    Type { basetype: target.ret_type.basetype.clone(), shape: TypePattern::aud() }
                } else {
                    target.ret_type.clone()
                };
                out_ty
            }
            FoldFun::Apply { .. } => {
                unimplemented!("'partial application' fold not yet supported")
            }
        };

        //if !types_compatible(&neutral_ty, &ret_ty) {
        //    self.errors.push(InferenceError::FoldFunctionTypeMismatch {
        //        expected: neutral_ty.clone(),
        //        found: ret_ty,
        //    });
        //}

        ret_ty
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
            Id::Var(id) => id.ty.borrow()
                .clone()
                .expect("Id::Var referenced before its assignment was processed"),
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
    expected.basetype == provided.basetype &&
        shapes_compatible(&expected.shape, &provided.shape)
}

fn shapes_compatible(expected: &TypePattern, provided: &TypePattern) -> bool {
    match (expected.rank(), provided.rank()) {
        (Some(er), Some(pr)) => er == pr,
        (None, Some(_)) => true,
        (Some(_), None) => true,
        (None, None) => true,
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
            ShapeRel::Greater => any_strict = true,
            ShapeRel::Equal => {}
            ShapeRel::Less | ShapeRel::Incomparable => return false,
        }
    }

    any_strict
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeRel {
    Greater,
    Equal,
    Less,
    Incomparable,
}

fn shape_relation(a: &TypePattern, b: &TypePattern) -> ShapeRel {
    let a_axes = &a.0;
    let b_axes = &b.0;

    if a_axes.len() != b_axes.len() {
        return ShapeRel::Incomparable;
    }

    let a_ge_b = a_axes
        .iter()
        .zip(b_axes.iter())
        .all(|(a, b)| axis_more_or_equal(a, b));

    let b_ge_a = a_axes
        .iter()
        .zip(b_axes.iter())
        .all(|(a, b)| axis_more_or_equal(b, a));

    match (a_ge_b, b_ge_a) {
        (true, true) => ShapeRel::Equal,
        (true, false) => ShapeRel::Greater,
        (false, true) => ShapeRel::Less,
        (false, false) => ShapeRel::Incomparable,
    }
}

fn axis_more_or_equal(a: &AxisPattern, b: &AxisPattern) -> bool {
    match (a, b) {
        _ => false,
    }
}

fn type_requires_runtime_dispatch(ty: &Type) -> bool {
    ty.type_pattern()
        .into_iter()
        .flatten()
        .any(axis_requires_runtime_dispatch)
}

fn axis_requires_runtime_dispatch(axis: &AxisPattern) -> bool {
    true
}
