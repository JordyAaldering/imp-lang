use std::collections::{HashMap, HashSet};

use crate::{ast::*, Rewrite};

/// Does not yet do anything.
pub fn analyse_tp(mut program: Program<'static, ParsedAst>) -> Program<'static, ParsedAst> {
    AnalyseTp::new().rewrite_program(&mut program);
    program
}

struct AnalyseTp {
    /// Symbols that have been defined so far in the current fundef,
    /// accumulated left-to-right across arguments and their type patterns.
    defined: HashSet<String>,
    symbol_terms: HashMap<String, ShapeTerm>,
}

impl AnalyseTp {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            symbol_terms: HashMap::new(),
        }
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'static VarInfo<'static, ParsedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa: () }))
    }

    fn alloc_expr(&self, expr: Expr<'static, ParsedAst>) -> &'static Expr<'static, ParsedAst> {
        Box::leak(Box::new(expr))
    }

    fn arg_expr(&self, arg_index: usize) -> &'static Expr<'static, ParsedAst> {
        self.alloc_expr(Expr::Id(Id::Arg(arg_index)))
    }

    fn shape_of_arg_expr(&self, arg_index: usize) -> Expr<'static, ParsedAst> {
        Expr::PrfCall(PrfCall::ShapeA(self.arg_expr(arg_index)))
    }

    fn dim_of_arg_expr(&self, arg_index: usize) -> Expr<'static, ParsedAst> {
        Expr::PrfCall(PrfCall::DimA(self.arg_expr(arg_index)))
    }

    fn dim_at_expr(&self, arg_index: usize, axis_index: usize) -> Expr<'static, ParsedAst> {
        let idx = self.alloc_expr(Expr::Const(Const::Usize(axis_index)));
        let idx_vec = self.alloc_expr(Expr::Array(Array { elems: vec![idx] }));
        let shp = self.alloc_expr(self.shape_of_arg_expr(arg_index));
        Expr::PrfCall(PrfCall::SelVxA(idx_vec, shp))
    }

    fn bind_symbol(
        &mut self,
        fundef: &mut Fundef<'static, ParsedAst>,
        symbol: &str,
        term: ShapeTerm,
        expr: Expr<'static, ParsedAst>,
        ty: Type,
    ) {
        if self.defined.insert(symbol.to_owned()) {
            self.symbol_terms.insert(symbol.to_owned(), term.clone());

            let lhs = self.alloc_lvis(symbol.to_owned(), Some(ty));
            let expr = self.alloc_expr(expr);
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

    fn analyse_arg_patterns(&mut self, fundef: &mut Fundef<'static, ParsedAst>) {
        let mut pending: Vec<(String, ShapeTerm, Expr<'static, ParsedAst>, Type)> = Vec::new();

        for (arg_index, arg) in fundef.args.iter().enumerate() {
            let TypePattern::Axes(axes) = &arg.ty.shape else {
                continue;
            };

            for (axis_index, axis) in axes.iter().enumerate() {
                match axis {
                    AxisPattern::Dim(DimPattern::Var(var)) => {
                        let term = ShapeTerm::ArgDim { arg_index, axis_index };
                        let expr = self.dim_at_expr(arg_index, axis_index);
                        pending.push((var.clone(), term, expr, Type::scalar(BaseType::Usize)));
                    }
                    AxisPattern::Rank(capture) => {
                        let dim_term = ShapeTerm::ArgRank {
                            arg_index,
                            axis_index,
                        };
                        let dim_expr = self.dim_of_arg_expr(arg_index);
                        pending.push((
                            capture.dim_name.clone(),
                            dim_term,
                            dim_expr,
                            Type::scalar(BaseType::Usize),
                        ));

                        let shp_term = ShapeTerm::TailShape {
                            arg_index,
                            start_axis: axis_index,
                        };
                        let shp_expr = self.shape_of_arg_expr(arg_index);
                        pending.push((
                            capture.shp_name.clone(),
                            shp_term,
                            shp_expr,
                            Type {
                                ty: BaseType::Usize,
                                shape: TypePattern::Any,
                            },
                        ));
                    }
                    AxisPattern::Dim(DimPattern::Any) | AxisPattern::Dim(DimPattern::Known(_)) => {}
                }
            }
        }

        for (symbol, term, expr, ty) in pending {
            self.bind_symbol(fundef, &symbol, term, expr, ty);
        }
    }

    fn analyse_ret_constraints(&mut self, fundef: &mut Fundef<'static, ParsedAst>) {
        let TypePattern::Axes(axes) = &fundef.ret_type.shape else {
            return;
        };

        let mut unconstrained_rank_captures = 0usize;

        for (axis_index, axis) in axes.iter().enumerate() {
            match axis {
                AxisPattern::Dim(DimPattern::Var(var)) => {
                    let constrained_by = if self.defined.contains(var) {
                        vec![ShapeTerm::Symbol(var.clone())]
                    } else {
                        Vec::new()
                    };

                    fundef.shape_facts.output_constraints.push(OutputShapeConstraint {
                        output: ShapeTerm::RetDim { axis_index },
                        constrained_by,
                    });
                }
                AxisPattern::Rank(capture) => {
                    let constrained_by = if self.defined.contains(&capture.dim_name) {
                        vec![ShapeTerm::Symbol(capture.dim_name.clone())]
                    } else {
                        unconstrained_rank_captures += 1;
                        Vec::new()
                    };

                    fundef.shape_facts.output_constraints.push(OutputShapeConstraint {
                        output: ShapeTerm::RetRank { axis_index },
                        constrained_by,
                    });
                }
                AxisPattern::Dim(DimPattern::Any) | AxisPattern::Dim(DimPattern::Known(_)) => {}
            }
        }

        fundef.shape_facts.unconstrained_rank_captures = unconstrained_rank_captures;
    }
}

impl Rewrite<'static> for AnalyseTp {
    type Ast = ParsedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'static, ParsedAst>) {
        self.defined.clear();
        self.symbol_terms.clear();

        fundef.shape_prelude.clear();
        fundef.shape_facts = ShapeFacts::default();

        for arg in &mut fundef.args {
            *arg = self.rewrite_farg(arg.clone());
        }

        fundef.ret_type = self.rewrite_type(fundef.ret_type.clone());

        self.analyse_arg_patterns(fundef);
        self.analyse_ret_constraints(fundef);
    }

    fn rewrite_farg(&mut self, arg: Farg) -> Farg {
        arg
    }

    fn rewrite_type(&mut self, ty: Type) -> Type {
        ty
    }
}
