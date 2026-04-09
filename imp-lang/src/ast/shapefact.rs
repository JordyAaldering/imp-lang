use super::*;

#[derive(Clone, Debug)]
pub struct ShapeFacts<'ast, Ast: AstConfig> {
    pub bindings: Vec<ShapeBinding<'ast, Ast>>,
    pub equalities: Vec<ShapeEquality>,
    pub output_constraints: Vec<OutputShapeConstraint>,
    pub unconstrained_rank_captures: usize,
}

impl<'ast, Ast: AstConfig> Default for ShapeFacts<'ast, Ast> {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
            equalities: Vec::new(),
            output_constraints: Vec::new(),
            unconstrained_rank_captures: 0,
        }
    }
}

impl<'ast, Ast: AstConfig> ShapeFacts<'ast, Ast> {
    pub fn map_stage<NextAst: AstConfig>(self) -> ShapeFacts<'ast, NextAst> {
        ShapeFacts {
            bindings: self
                .bindings
                .into_iter()
                .map(|binding| ShapeBinding {
                    symbol: binding.symbol,
                    term: binding.term,
                    source: None,
                })
                .collect(),
            equalities: self.equalities,
            output_constraints: self.output_constraints,
                unconstrained_rank_captures: self.unconstrained_rank_captures,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShapeBinding<'ast, Ast: AstConfig> {
    pub symbol: String,
    pub term: ShapeTerm,
    pub source: Option<&'ast VarInfo<'ast, Ast>>,
}

#[derive(Clone, Debug)]
pub struct ShapeEquality {
    pub left: ShapeTerm,
    pub right: ShapeTerm,
}

#[derive(Clone, Debug)]
pub struct OutputShapeConstraint {
    pub output: ShapeTerm,
    pub constrained_by: Vec<ShapeTerm>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShapeTerm {
    Known(usize),
    Symbol(String),
    ArgDim { arg_index: usize, axis_index: usize },
    ArgRank { arg_index: usize, axis_index: usize },
    RetDim { axis_index: usize },
    RetRank { axis_index: usize },
    TailShape { arg_index: usize, start_axis: usize },
}