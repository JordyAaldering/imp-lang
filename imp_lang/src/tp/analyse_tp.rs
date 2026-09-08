use std::collections::{HashMap, HashSet};

use crate::{Phase, ast::*, phase::phase_log};

pub fn analyse_tp<'ast>(program: &mut Program<'ast, ParsedAst>, scope: &'ast Scope<'ast, ParsedAst>) {
    AnalyseTp::new(scope).trav_program(program);
}

struct AnalyseTp<'ast> {
    scope: &'ast Scope<'ast, ParsedAst>,
    /// Symbols that have been defined so far in the current fundef,
    /// accumulated left-to-right across arguments and their type patterns.
    defined: HashSet<String>,
    symbol_terms: HashMap<String, ShapeTerm>,
    arg_index: usize,
}

impl<'ast> AnalyseTp<'ast> {
    fn new(scope: &'ast Scope<'ast, ParsedAst>) -> Self {
        Self {
            scope,
            defined: HashSet::new(),
            symbol_terms: HashMap::new(),
            arg_index: 0,
        }
    }

    fn alloc_avis(&self, fundef: &mut Fundef<'ast, ParsedAst>, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, ParsedAst> {
        let var = self.scope.alloc_avis(name, ty, ());
        fundef.decs.push(var);
        var
    }

    fn shape_of_arg_expr(&self, arg_index: usize) -> Expr<'ast, ParsedAst> {
        let arg_id = Expr::Id(Id::Arg(arg_index));
        Expr::Prf(Prf::ShapeA(self.scope.alloc_expr(arg_id)))
    }

    fn dim_of_arg_expr(&self, arg_index: usize) -> Expr<'ast, ParsedAst> {
        let arg_id = Expr::Id(Id::Arg(arg_index));
        Expr::Prf(Prf::DimA(self.scope.alloc_expr(arg_id)))
    }

    fn dim_at_expr(&self, arg_index: usize, axis_index: usize) -> Expr<'ast, ParsedAst> {
        let idx = self.scope.alloc_expr(Expr::Const(Const::Usize(axis_index)));
        let idx_vec = self.scope.alloc_expr(Expr::Array(Array { elems: vec![idx] }));
        let shp = self.scope.alloc_expr(self.shape_of_arg_expr(arg_index));
        Expr::Prf(Prf::SelVxA(idx_vec, shp))
    }

    fn bind_symbol(
        &mut self,
        fundef: &mut Fundef<'ast, ParsedAst>,
        symbol: &str,
        term: ShapeTerm,
        expr: Expr<'ast, ParsedAst>,
        ty: Type,
    ) {
        if self.defined.insert(symbol.to_owned()) {
            self.symbol_terms.insert(symbol.to_owned(), term.clone());

            let lhs = self.alloc_avis(fundef, symbol.to_owned(), Some(ty));
            let expr = self.scope.alloc_expr(expr);
            fundef.shape_prelude.push(Assign { lhs, expr });
            fundef.shape_facts.bindings.push(ShapeBinding {
                symbol: symbol.to_owned(),
                term,
            });
        } else {
            fundef.shape_facts.equalities.push(ShapeEquality {
                left: ShapeTerm::Symbol(symbol.to_owned()),
                right: term,
            });
        }
    }

    fn analyse_ret_constraints(&mut self, fundef: &mut Fundef<'ast, ParsedAst>) -> usize {
        let Some(axes) = fundef.ret_type.type_pattern() else {
            return 0;
        };

        let mut unconstrained_rank_captures = 0usize;

        for (axis_index, axis) in axes.iter().enumerate() {
            match axis {
                AxisPattern::ShapePattern { dim, shp: _ } => {
                    let constrained_by = if let RankCapture::Var(dim) = dim && self.defined.contains(dim) {
                        vec![ShapeTerm::Symbol(dim.clone())]
                    } else {
                        unconstrained_rank_captures += 1;
                        Vec::new()
                    };

                    fundef.shape_facts.output_constraints.push(OutputShapeConstraint {
                        output: ShapeTerm::RetRank { axis_index },
                        constrained_by,
                    });
                },
                AxisPattern::DimPattern { len: RankCapture::Fixed(_) } => {
                    fundef.shape_facts.output_constraints.push(OutputShapeConstraint {
                        output: ShapeTerm::RetDim { axis_index },
                        constrained_by: Vec::new(),
                    });
                },
                AxisPattern::DimPattern { len: RankCapture::Var(len) } => {
                    let constrained_by = if self.defined.contains(len) {
                        vec![ShapeTerm::Symbol(len.clone())]
                    } else {
                        Vec::new()
                    };

                    fundef.shape_facts.output_constraints.push(OutputShapeConstraint {
                        output: ShapeTerm::RetDim { axis_index },
                        constrained_by,
                    });
                }
                AxisPattern::DimPattern { len: RankCapture::Free } => {},
            }
        }

        unconstrained_rank_captures
    }
}

#[derive(Default)]
struct PendingTerms<'ast>(Vec<(String, ShapeTerm, Expr<'ast, ParsedAst>, Type)>);

impl<'ast> FromIterator<PendingTerms<'ast>> for PendingTerms<'ast> {
    fn from_iter<T: IntoIterator<Item = PendingTerms<'ast>>>(iter: T) -> Self {
        let terms: Vec<_> = iter.into_iter()
            .flat_map(|x| x.0)
            .collect();
        Self(terms)
    }
}

impl<'ast> Traverse<'ast> for AnalyseTp<'ast> {
	const PHASE: Phase = Phase::ATP;

    type Ast = ParsedAst;

    type DeclOut = PendingTerms<'ast>;

    type ExprOut = ();

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, ParsedAst>) {
        phase_log!("Analysing type patterns of {}", fundef.name);

        self.defined.clear();
        self.symbol_terms.clear();

        fundef.shape_prelude.clear();
        fundef.shape_facts = ShapeFacts::default();

        // Arguments
        {
            let pending = self.trav_fargs(&mut fundef.args).0;

            for (symbol, term, expr, ty) in pending {
                self.bind_symbol(fundef, &symbol, term, expr, ty);
            }
        }

        // Return type
        {
            let unconstrained_rank_captures = self.analyse_ret_constraints(fundef);
            fundef.shape_facts.unconstrained_rank_captures = unconstrained_rank_captures;
        }
    }

    fn trav_fargs(&mut self, args: &mut [Farg]) -> Self::DeclOut {
        args.iter_mut()
            .enumerate()
            .map(|(i, arg)| {
                self.arg_index = i;
                self.trav_farg(arg)
            })
            .collect()
    }

    fn trav_farg(&mut self, arg: &mut Farg) -> Self::DeclOut {
        let Some(axes) = arg.ty.type_pattern() else {
            return Default::default();
        };

        let mut pending = Vec::new();

        for (axis_index, axis) in axes.iter().enumerate() {
            match axis {
                AxisPattern::ShapePattern { dim, shp } => {
                    if let RankCapture::Var(dim) = dim {
                        let dim_term = ShapeTerm::ArgRank {
                            arg_index: self.arg_index,
                            axis_index,
                        };
                        let dim_expr = self.dim_of_arg_expr(self.arg_index);
                        pending.push((
                            dim.clone(),
                            dim_term,
                            dim_expr,
                            Type::scalar(BaseType::Usize),
                        ));
                    }

                    if let Some(shp) = shp {
                        let shp_term = ShapeTerm::TailShape {
                            arg_index: self.arg_index,
                            start_axis: axis_index,
                        };
                        let shp_expr = self.shape_of_arg_expr(self.arg_index);
                        pending.push((
                            shp.clone(),
                            shp_term,
                            shp_expr,
                            Type {
                                basetype: BaseType::Usize,
                                shape: TypePattern::scalar(),
                            },
                        ));
                    }
                },
                AxisPattern::DimPattern { len: RankCapture::Var(len) } => {
                    let term = ShapeTerm::ArgDim {
                        arg_index: self.arg_index,
                        axis_index,
                    };
                    let expr = self.dim_at_expr(self.arg_index, axis_index);
                    pending.push((len.clone(), term, expr, Type::scalar(BaseType::Usize)));
                }
                AxisPattern::DimPattern { .. } => {}
            }
        }

        PendingTerms(pending)
    }
}
