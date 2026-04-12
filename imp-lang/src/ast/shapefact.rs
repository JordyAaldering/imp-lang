#[derive(Clone, Debug, Default)]
pub struct ShapeFacts {
    pub bindings: Vec<ShapeBinding>,
    pub equalities: Vec<ShapeEquality>,
    pub output_constraints: Vec<OutputShapeConstraint>,
    pub unconstrained_rank_captures: usize,
}

#[derive(Clone, Debug)]
pub struct ShapeBinding {
    pub symbol: String,
    pub term: ShapeTerm,
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