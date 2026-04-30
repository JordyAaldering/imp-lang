use std::{collections::HashMap, mem};

use typed_arena::Arena;

use crate::ast::*;

pub fn resolve_dispatch<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, DispatchError> {
    let mut out_program = Program {
        overloads: HashMap::new(),
        fundefs: Arena::new(),
    };

    let mut overloads: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>> = HashMap::new();
    let mut work_items: Vec<(*mut Fundef<'ast, TypedAst>, &'ast Fundef<'ast, UntypedAst>)> = Vec::new();

    for (name, groups) in &program.overloads {
        let mut out_groups = HashMap::new();
        for (sig, fundefs) in groups {
            let mut out_fundefs = Vec::new();
            for fundef in fundefs {
                let stub = out_program.fundefs.alloc(Fundef {
                    name: fundef.name.clone(),
                    ret_type: fundef.ret_type.clone(),
                    args: fundef.args.clone(),
                    shape_prelude: Vec::new(),
                    shape_facts: fundef.shape_facts.clone(),
                    decs: Arena::new(),
                    exprs: Arena::new(),
                    body: Body {
                        stmts: Vec::new(),
                        ret: Id::Arg(usize::MAX),
                    },
                });
                let stub_ptr = stub as *mut Fundef<'ast, TypedAst>;
                let stub_ref: &'ast Fundef<'ast, TypedAst> = unsafe { std::mem::transmute(stub) };
                out_fundefs.push(stub_ref);
                work_items.push((stub_ptr, *fundef));
            }
            out_groups.insert(sig.clone(), out_fundefs);
        }
        overloads.insert(name.clone(), out_groups);
    }

    for (slot_ptr, src_fundef) in work_items {
        let mut lower = DispatchResolver::new(overloads.clone());
        let lowered = lower.lower_fundef(src_fundef);
        if let Some(err) = lower.errors.into_iter().next() {
            return Err(err);
        }
        unsafe {
            std::ptr::replace(slot_ptr, lowered);
        }
    }

    out_program.overloads = overloads;
    Ok(out_program)
}

#[allow(unused)]
#[derive(Debug)]
pub enum DispatchError {
    MissingTypeAnnotation { name: String },
    UndefinedFunction { name: String },
    NoMatchingOverload { name: String, arg_bases: BaseSignature },
    AmbiguousOverload { name: String, arg_bases: BaseSignature },
}

struct DispatchResolver<'ast> {
    args: Vec<Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    decs_arena: Arena<VarInfo<'ast, TypedAst>>,
    expr_arena: Arena<Expr<'ast, TypedAst>>,
    errors: Vec<DispatchError>,
    overloads: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>>,
}

impl<'ast> DispatchResolver<'ast> {
    fn new(overloads: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, TypedAst>>>>) -> Self {
        Self {
            args: Vec::new(),
            idmap: HashMap::new(),
            decs_arena: Arena::new(),
            expr_arena: Arena::new(),
            errors: Vec::new(),
            overloads,
        }
    }

    fn alloc_lvis(&self, name: String, ty: Type, ssa: Option<&'ast Expr<'ast, TypedAst>>) -> &'ast VarInfo<'ast, TypedAst> {
        unsafe { std::mem::transmute(self.decs_arena.alloc(VarInfo { name, ty, ssa })) }
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
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

    fn resolve_target(&mut self, func_name: &str, arg_types: &[Type]) -> &'ast Fundef<'ast, TypedAst> {
        let Some(group) = self.overloads.get(func_name) else {
            self.errors.push(DispatchError::UndefinedFunction {
                name: func_name.to_owned(),
            });
            panic!("undefined function during dispatch resolution: {}", func_name);
        };

        let key = BaseSignature {
            base_types: arg_types.iter().map(|ty| ty.ty.clone()).collect(),
        };

        let Some(candidates) = group.get(&key) else {
            self.errors.push(DispatchError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no matching overload during dispatch resolution: {}", func_name);
        };

        let mut matches = Vec::new();
        for target in candidates {
            let mut ok = true;
            for (expected, provided) in target.args.iter().zip(arg_types.iter()) {
                if !types_compatible(&expected.ty, provided) {
                    ok = false;
                    break;
                }
            }
            if ok {
                matches.push(*target);
            }
        }

        if matches.is_empty() {
            self.errors.push(DispatchError::NoMatchingOverload {
                name: func_name.to_owned(),
                arg_bases: key.clone(),
            });
            panic!("no compatible overload during dispatch resolution: {}", func_name);
        }

        let best = maximal_candidates(&matches);
        if best.len() > 1 && !arg_types.iter().any(type_requires_runtime_dispatch) {
            self.errors.push(DispatchError::AmbiguousOverload {
                name: func_name.to_owned(),
                arg_bases: key,
            });
        }

        best[0]
    }

    fn lower_fundef(&mut self, fundef: &Fundef<'ast, UntypedAst>) -> Fundef<'ast, TypedAst> {
        self.args = fundef.args.clone();
        self.idmap.clear();
        self.decs_arena = Arena::new();
        self.expr_arena = Arena::new();

        let mut shape_prelude = Vec::new();
        for assign in &fundef.shape_prelude {
            shape_prelude.push(self.lower_assign(*assign));
        }

        let body = self.lower_body(fundef.body.clone());

        let decs = mem::take(&mut self.decs_arena);
        let exprs = mem::take(&mut self.expr_arena);

        Fundef {
            name: fundef.name.clone(),
            ret_type: fundef.ret_type.clone(),
            args: fundef.args.clone(),
            shape_prelude,
            shape_facts: fundef.shape_facts.clone(),
            decs,
            exprs,
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
        let expr = self.lower_expr((*assign.expr).clone());
        let expr_ref = self.alloc_expr(expr);
        let lhs_ty = self.require_ty(&assign.lhs.name, &assign.lhs.ty);
        let lhs = self.alloc_lvis(assign.lhs.name.clone(), lhs_ty, Some(expr_ref));
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
            id: CallTarget::Function(target),
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
                FoldFun::Name(CallTarget::Function(target))
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
        let iv_ty = self.require_ty(&tensor.iv.name, &tensor.iv.ty);
        let iv = self.alloc_lvis(tensor.iv.name.clone(), iv_ty, None);
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

fn maximal_candidates<'ast>(candidates: &[&'ast Fundef<'ast, TypedAst>]) -> Vec<&'ast Fundef<'ast, TypedAst>> {
    let mut maximal: Vec<&Fundef<'_, TypedAst>> = Vec::new();

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
        (TypePattern::Scalar, TypePattern::Axes(axes)) => axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))),
        (TypePattern::Axes(a_axes), TypePattern::Scalar) => a_axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))),
        (TypePattern::Axes(a_axes), TypePattern::Axes(b_axes)) => axes_more_or_equal(a_axes, b_axes),
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

fn types_compatible(expected: &Type, provided: &Type) -> bool {
    expected.ty == provided.ty && shapes_compatible(&expected.shape, &provided.shape)
}

fn shapes_compatible(expected: &TypePattern, provided: &TypePattern) -> bool {
    let has_rank = |axes: &[AxisPattern]| axes.iter().any(|a| matches!(a, AxisPattern::Rank(_)));
    match (expected, provided) {
        (TypePattern::Scalar, TypePattern::Scalar) => true,
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

fn type_requires_runtime_dispatch(ty: &Type) -> bool {
    match &ty.shape {
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
