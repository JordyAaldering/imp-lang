use std::collections::HashSet;

use crate::{ast::*, Rewrite};

pub fn analyse_tp(mut program: Program<'static, ParsedAst>) -> Program<'static, ParsedAst> {
    AnalyseTp::new().rewrite_program(&mut program);
    program
}

struct AnalyseTp {
    /// Symbols that have been defined so far in the current fundef,
    /// accumulated left-to-right across arguments and their type patterns.
    defined: HashSet<String>,
}

impl AnalyseTp {
    fn new() -> Self {
        Self { defined: HashSet::new() }
    }
}

impl Rewrite<'static> for AnalyseTp {
    type Ast = ParsedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'static, ParsedAst>) {
        self.defined.clear();
        // Process arguments left-to-right so each arg sees symbols defined by
        // all preceding scalar args and type patterns.
        let new_args = fundef.args.iter().map(|&arg| self.rewrite_farg(arg)).collect();
        fundef.args = new_args;
        fundef.ret_type = self.rewrite_type(fundef.ret_type.clone());
        // Body statements do not contain type patterns; no further rewriting needed.
    }

    fn rewrite_farg(&mut self, arg: &'static Farg) -> &'static Farg {
        let ty = self.rewrite_type(arg.ty.clone());
        // Scalar arguments make their name available as a dimension symbol for
        // all subsequent argument type patterns and the return type.
        if ty.is_scalar() {
            self.defined.insert(arg.name.clone());
        }
        Box::leak(Box::new(Farg { name: arg.name.clone(), ty }))
    }

    fn rewrite_type(&mut self, mut ty: Type) -> Type {
        resolve_shape_roles(&mut ty.shape, &mut self.defined);
        ty.knowledge = knowledge_from_shape(&ty.shape);
        ty
    }
}

/// Walk the axes of a shape and fix up `SymbolRole`s using the current
/// `defined` set.  New `Define` roles are inserted into `defined`.
fn resolve_shape_roles(shape: &mut ShapePattern, defined: &mut HashSet<String>) {
    let ShapePattern::Axes(axes) = shape else { return };
    for axis in axes.iter_mut() {
        match axis {
            AxisPattern::Dim(DimPattern::Var(var)) => {
                var.role = resolve_role(&var.name, defined);
            }
            AxisPattern::Rank(capture) => {
                capture.dim_role = resolve_role(&capture.dim_name, defined);
                // shp_name is always a fresh binding introduced by this pattern.
                defined.insert(capture.shp_name.clone());
            }
            _ => {}
        }
    }
}

fn resolve_role(name: &str, defined: &mut HashSet<String>) -> SymbolRole {
    if defined.contains(name) {
        SymbolRole::Use
    } else {
        defined.insert(name.to_owned());
        SymbolRole::Define
    }
}

/// Derive the `TypeKnowledge` value from an already role-resolved `ShapePattern`.
fn knowledge_from_shape(shape: &ShapePattern) -> TypeKnowledge {
    match shape {
        ShapePattern::Scalar => TypeKnowledge::Scalar,
        ShapePattern::Any => TypeKnowledge::AUD,
        ShapePattern::Axes(axes) => {
            let has_rest = axes.iter().any(|a| matches!(a, AxisPattern::Rank(_)));
            if has_rest {
                let min_rank = axes.iter()
                    .filter(|a| matches!(a, AxisPattern::Dim(_)))
                    .count() as u8;
                return TypeKnowledge::AUDGN { min_rank };
            }
            if axes.iter().any(|a| matches!(a, AxisPattern::Dim(DimPattern::Any))) {
                TypeKnowledge::AKD
            } else {
                TypeKnowledge::AKS
            }
        }
    }
}
