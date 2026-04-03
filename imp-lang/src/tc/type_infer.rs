use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    let mut infer = TypeInfer::new();
    let typed = infer.trav_program(program);
    if let Some(err) = infer.errors.into_iter().next() {
        Err(err)
    } else {
        Ok(typed)
    }
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_ids: Vec<&'ast VarInfo<'ast, TypedAst>>,
    errors: Vec<InferenceError>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum InferenceError {
    SelectionIndexNotScalar {
        idx: usize,
        ty: Type,
    },
    SelectionIndexNotInteger {
        idx: usize,
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
}

impl<'ast> TypeInfer<'ast> {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            idmap: HashMap::new(),
            new_ids: Vec::new(),
            errors: Vec::new(),
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

    fn shape_knowledge(shape: &ShapePattern) -> TypeKnowledge {
        match shape {
            ShapePattern::Scalar => TypeKnowledge::Scalar,
            ShapePattern::Any => TypeKnowledge::AUD,
            ShapePattern::Axes(axes) => {
                let has_rest = axes.iter().any(|a| matches!(a, AxisPattern::Rest(_)));
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

    fn selection_result_shape(&self, arr_shape: &ShapePattern, scalar_idx_count: usize) -> Result<ShapePattern, InferenceError> {
        match arr_shape {
            ShapePattern::Scalar => {
                if scalar_idx_count == 0 {
                    Ok(ShapePattern::Scalar)
                } else {
                    Err(InferenceError::SelectionRankTooSmall {
                        needed: scalar_idx_count,
                        known_min_rank: Some(0),
                        shape: arr_shape.clone(),
                    })
                }
            }
            ShapePattern::Any => {
                if scalar_idx_count == 0 {
                    Ok(ShapePattern::Any)
                } else {
                    Err(InferenceError::SelectionRankTooSmall {
                        needed: scalar_idx_count,
                        known_min_rank: None,
                        shape: arr_shape.clone(),
                    })
                }
            }
            ShapePattern::Axes(axes) => {
                let min_rank = axes.iter().filter(|a| matches!(a, AxisPattern::Dim(_))).count();
                if scalar_idx_count > min_rank {
                    return Err(InferenceError::SelectionRankTooSmall {
                        needed: scalar_idx_count,
                        known_min_rank: Some(min_rank),
                        shape: arr_shape.clone(),
                    });
                }

                let has_rest = axes.iter().any(|a| matches!(a, AxisPattern::Rest(_)));
                if !has_rest {
                    let rem: Vec<AxisPattern> = axes.iter().skip(scalar_idx_count).cloned().collect();
                    if rem.is_empty() {
                        Ok(ShapePattern::Scalar)
                    } else {
                        Ok(ShapePattern::Axes(rem))
                    }
                } else {
                    let rest_pos = axes.iter().position(|a| matches!(a, AxisPattern::Rest(_))).unwrap();
                    if scalar_idx_count < rest_pos {
                        Ok(ShapePattern::Axes(axes.iter().skip(scalar_idx_count).cloned().collect()))
                    } else if scalar_idx_count == rest_pos {
                        Ok(ShapePattern::Axes(axes.iter().skip(rest_pos).cloned().collect()))
                    } else {
                        // Crossing into `..rest` loses exact residual shape information.
                        Ok(ShapePattern::Any)
                    }
                }
            }
        }
    }

    /// Build the type of an array literal. The element count becomes a leading `Known(n)` dimension
    /// prepended to the element type's shape. Errors on base-type or rank mismatch between elements.
    fn array_literal_type(&mut self, elem_types: Vec<Type>) -> Type {
        let count = elem_types.len();

        let Some(first) = elem_types.first() else {
            // Empty literal: return u32[0] as a conservative scalar-element vector.
            return Type::vector_dim(BaseType::U32, DimPattern::Known(0));
        };

        let base_ty = first.ty;
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

        let leading = AxisPattern::Dim(DimPattern::Known(count as u64));
        let result_shape = match &elem_shape {
            ShapePattern::Scalar => ShapePattern::Axes(vec![leading]),
            ShapePattern::Axes(axes) => {
                let mut new_axes = Vec::with_capacity(1 + axes.len());
                new_axes.push(leading);
                new_axes.extend_from_slice(axes);
                ShapePattern::Axes(new_axes)
            }
            // Element shape is fully unknown; can only say rank >= 1.
            ShapePattern::Any => ShapePattern::Axes(vec![leading, AxisPattern::Rest(RestPattern {
                name: "_rest".to_owned(),
                role: SymbolRole::Define,
            })]),
        };

        let knowledge = Self::shape_knowledge(&result_shape);
        Type { ty: base_ty, shape: result_shape, knowledge }
    }

    fn selection_result_type(&self, arr_ty: &Type, scalar_idx_count: usize) -> Result<Type, InferenceError> {
        let shape = self.selection_result_shape(&arr_ty.shape, scalar_idx_count)?;
        let knowledge = Self::shape_knowledge(&shape);
        Ok(Type {
            ty: arr_ty.ty,
            shape,
            knowledge,
        })
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    ///
    /// Declarations
    ///

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        let Fundef { name, ret_type, args, body, decs: _ } = fundef;

        self.args = args.clone();
        self.idmap.clear();
        self.new_ids.clear();

        let new_args: Vec<&'ast Farg> = args.into_iter().map(|arg| self.trav_farg(arg)).collect();

        let mut new_body = Vec::new();
        for stmt in body {
            new_body.push(self.trav_stmt(stmt));
        }

        Fundef {
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

    ///
    /// Statements
    ///

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let (new_expr, new_ty) = self.trav_expr((*assign.expr).clone());
        let expr_ref = self.alloc_expr(new_expr);
        let new_lvis = self.alloc_lvis(assign.lvis.name.clone(), new_ty, Some(expr_ref));
        self.idmap.insert(assign.lvis as *const _, new_lvis);
        self.new_ids.push(new_lvis);
        Assign { lvis: new_lvis, expr: expr_ref }
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
            }
            Binary(n) => {
                let (expr, ty) = self.trav_binary(n);
                (Binary(expr), ty)
            }
            Unary(n) => {
                let (expr, ty) = self.trav_unary(n);
                (Unary(expr), ty)
            }
            Array(n) => {
                let (expr, ty) = self.trav_array(n);
                (Array(expr), ty)
            }
            Sel(n) => {
                let (expr, ty) = self.trav_sel(n);
                (Sel(expr), ty)
            }
            Id(n) => {
                let (id, ty) = self.trav_id(n);
                (Id(id), ty)
            }
            Bool(n) => {
                let (expr, ty) = self.trav_bool(n);
                (Bool(expr), ty)
            }
            U32(n) => {
                let (expr, ty) = self.trav_u32(n);
                (U32(expr), ty)
            }
        }
    }

    type TensorOut = (Tensor<'ast, Self::OutAst>, Type);

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        let (lb, _) = self.trav_id(tensor.lb);
        let (ub, _) = self.trav_id(tensor.ub);

        let iv_new = self.alloc_lvis(tensor.iv.name.clone(), Type::scalar(BaseType::U32), None);
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);

        let mut body = Vec::new();
        for stmt in tensor.body {
            body.push(self.trav_stmt(stmt));
        }

        let (ret, _ret_ty) = self.trav_id(tensor.ret);

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

    type ArrayOut = (Array<'ast, Self::OutAst>, Type);

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut {
        let mut values = Vec::with_capacity(array.values.len());
        let mut elem_types = Vec::with_capacity(array.values.len());

        for value in array.values {
            let (value, ty) = self.trav_id(value);
            elem_types.push(ty);
            values.push(value);
        }

        let ty = self.array_literal_type(elem_types);
        (Array { values }, ty)
    }

    type SelOut = (Sel<'ast, Self::OutAst>, Type);

    fn trav_sel(&mut self, sel: Sel<'ast, Self::InAst>) -> Self::SelOut {
        let (arr, arr_ty) = self.trav_id(sel.arr);

        let mut idxs = Vec::with_capacity(sel.idx.len());
        for (idx_pos, idx) in sel.idx.into_iter().enumerate() {
            let (idx, idx_ty) = self.trav_id(idx);
            if !idx_ty.is_scalar() {
                self.errors.push(InferenceError::SelectionIndexNotScalar { idx: idx_pos, ty: idx_ty.clone() });
            } else if !matches!(idx_ty.ty, BaseType::U32 | BaseType::Usize) {
                self.errors.push(InferenceError::SelectionIndexNotInteger { idx: idx_pos, ty: idx_ty.clone() });
            }
            idxs.push(idx);
        }

        let ty = match self.selection_result_type(&arr_ty, idxs.len()) {
            Ok(ty) => ty,
            Err(err) => {
                self.errors.push(err);
                // Continue inference with a conservative fallback type to preserve traversal.
                Type {
                    ty: arr_ty.ty,
                    shape: ShapePattern::Any,
                    knowledge: TypeKnowledge::AUD,
                }
            }
        };

        (Sel { arr, idx: idxs }, ty)
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
                let new_id = self.idmap.get(&(old as *const _))
                    .expect("Id::Var referenced before its assignment was processed");
                let ty = new_id.ty.clone();
                (Id::Var(*new_id), ty)
            },
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
