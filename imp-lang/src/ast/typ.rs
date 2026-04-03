#[derive(Clone, Debug)]
pub struct Type {
    pub ty: BaseType,
    pub pattern: Option<TypePattern>,
    pub knowledge: TypeKnowledge,
}

#[derive(Clone, Copy, Debug)]
pub enum BaseType {
    U32,
    Bool,
}

#[derive(Clone, Debug)]
pub struct TypePattern {
    /// Overall shape pattern of the type.
    /// Example: `u32[n]` uses `ShapePattern::Axes([AxisPattern::Dim(DimPattern::Var(ExtentVar { .. }))])`.
    pub shape: ShapePattern,
    /// Named symbols introduced in this type and their origin.
    /// Example: in `fn (u32[n] a, u32[n] b) -> u32[n]`, `n` can be bound to `a`'s dim 0.
    pub binds: Vec<PatternBinding>,
    /// Additional compile-time predicates over symbols.
    /// Example: `n == arg0.dim0`.
    pub constraints: Vec<PatternConstraint>,
}

#[derive(Clone, Debug)]
pub enum ShapePattern {
    /// Rank-0 shape.
    /// Surface example: `u32`.
    Scalar,
    /// Fully fixed rank and extents.
    /// Surface examples: `u32[42]`, `u32[n]`, `u32[len,u,..rest,v,w]`.
    Axes(Vec<AxisPattern>),
    /// Shape is unconstrained.
    /// Surface example: `u32[*]`.
    Any,
}

#[derive(Clone, Debug)]
pub enum RankPattern {
    /// Rank is unknown.
    /// Surface example: `u32[*]`.
    Any,
    /// Rank is exactly this value.
    /// Surface examples: `Exact(0)` for scalar `u32`, `Exact(1)` for vectors like `u32[n]`.
    Exact(u8),
}

#[derive(Clone, Debug)]
pub enum DimPattern {
    /// Dimension unconstrained.
    /// Surface example: `u32[.]` or `u32[_]`.
    Any,
    /// Dimension is a compile-time constant.
    /// Surface example: `u32[42]`.
    Known(u64),
    /// Dimension is a named variable symbol.
    /// Surface examples: `u32[n]`, `u32[len]`.
    Var(ExtentVar),
}

#[derive(Clone, Debug)]
pub struct ExtentVar {
    pub name: String,
    pub role: SymbolRole,
}

#[derive(Clone, Copy, Debug)]
pub enum SymbolRole {
    /// First occurrence in left-to-right argument order.
    Define,
    /// Reuse of an already defined symbol.
    Use,
}

#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// Single dimension entry (`42`, `n`, `len`).
    Dim(DimPattern),
    /// Variable-length remainder capture (`..rest`) which may appear between dimensions.
    Rest(RestPattern),
}

#[derive(Clone, Debug)]
pub struct RestPattern {
    pub name: String,
    pub role: SymbolRole,
}

#[derive(Clone, Debug)]
pub struct PatternBinding {
    /// Symbol introduced by this type pattern.
    /// Surface example: `n` in `u32[n]`.
    pub name: String,
    /// Where the symbol value comes from.
    pub source: PatternBindingSource,
}

#[derive(Clone, Debug)]
pub enum PatternBindingSource {
    /// Bind from the local dimension index.
    Dim(usize),
    /// Bind from a rest capture index in the axis pattern.
    Rest(usize),
}

#[derive(Clone, Debug)]
pub enum PatternConstraint {
    /// Two symbols must resolve to the same dimension.
    /// Surface example: `n == m`.
    SameSymbol {
        left: String,
        right: String,
    },
}

/// SaC-inspired array knowledge classes, extended with type-pattern payloads.
#[derive(Clone, Debug)]
pub enum TypeKnowledge {
    /// Scalar values are not in the SaC AK lattice.
    Scalar,
    /// Array Known Value: compile-time known values and shape pattern.
    /// Example: literal `[1, 2, 3]` has AKV with shape `[3]`.
    AKV(AkvInfo),
    /// Array Known Shape: compile-time known shape pattern.
    /// Example: argument `u32[n]` usually starts in AKS.
    AKS(AksInfo),
    /// Array Known Dimension: known rank, symbolic or partially known extents.
    /// Example: rank known to be 1 but extent unknown.
    AKD(AkdInfo),
    /// Array Unknown Dimension.
    /// Example: unconstrained array parameter.
    AUD,
    /// Array Unknown Dimension Greater than N.
    /// Example: `u32[m,n,..rest]` implies rank >= 2, so this can be represented as `AUDGN { min_rank: 2 }`.
    AUDGN { min_rank: u8 },
}

#[derive(Clone, Debug)]
pub struct AkvInfo {
    pub shape: ShapePattern,
    pub value_pattern: ValuePattern,
}

#[derive(Clone, Debug)]
pub struct AksInfo {
    pub shape: ShapePattern,
}

#[derive(Clone, Debug)]
pub struct AkdInfo {
    pub rank: RankPattern,
    pub dims: Vec<DimPattern>,
}

#[derive(Clone, Debug)]
pub enum ValuePattern {
    Any,
    Bool(bool),
    U32(u32),
    /// Opaque symbolic value expression (constant folding can refine this later).
    Symbolic(String),
}

impl Type {
    pub fn scalar(ty: BaseType) -> Self {
        Self {
            ty,
            pattern: Some(TypePattern {
                shape: ShapePattern::Scalar,
                binds: Vec::new(),
                constraints: Vec::new(),
            }),
            knowledge: TypeKnowledge::Scalar,
        }
    }

    pub fn vector(ty: BaseType, extent: &str) -> Self {
        let dim = if extent == "." {
            DimPattern::Any
        } else {
            DimPattern::Var(ExtentVar {
                name: extent.to_owned(),
                role: SymbolRole::Use,
            })
        };

        Self::vector_with_dim_pattern(ty, dim)
    }

    pub fn vector_with_dim_pattern(ty: BaseType, dim: DimPattern) -> Self {
        let shape = ShapePattern::Axes(vec![AxisPattern::Dim(dim)]);

        Self {
            ty,
            pattern: Some(TypePattern {
                shape: shape.clone(),
                binds: Vec::new(),
                constraints: Vec::new(),
            }),
            knowledge: TypeKnowledge::AKS(AksInfo { shape }),
        }
    }

    pub fn with_axes(mut self, axes: Vec<AxisPattern>) -> Self {
        self.pattern = Some(TypePattern {
            shape: ShapePattern::Axes(axes),
            binds: Vec::new(),
            constraints: Vec::new(),
        });
        self
    }

    pub fn bind_symbol_to_dim(mut self, symbol: &str, dim_index: usize) -> Self {
        if let Some(pattern) = &mut self.pattern {
            pattern.binds.push(PatternBinding {
                name: symbol.to_owned(),
                source: PatternBindingSource::Dim(dim_index),
            });
        }
        self
    }

    pub fn from_pattern(ty: BaseType, pattern: TypePattern, knowledge: TypeKnowledge) -> Self {
        Self {
            ty,
            pattern: Some(pattern),
            knowledge,
        }
    }

    pub fn with_pattern(mut self, pattern: TypePattern) -> Self {
        self.pattern = Some(pattern);
        self
    }

    pub fn with_knowledge(mut self, knowledge: TypeKnowledge) -> Self {
        self.knowledge = knowledge;
        self
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.pattern_shape(), Some(ShapePattern::Scalar))
    }

    pub fn is_vector(&self) -> bool {
        matches!(self.rank_pattern(), RankPattern::Exact(1))
    }

    pub fn pattern_shape(&self) -> Option<&ShapePattern> {
        self.pattern.as_ref().map(|p| &p.shape)
    }

    pub fn rank_pattern(&self) -> RankPattern {
        match self.pattern_shape() {
            Some(ShapePattern::Scalar) => RankPattern::Exact(0),
            Some(ShapePattern::Axes(axes)) => {
                if axes.iter().any(|axis| matches!(axis, AxisPattern::Rest(_))) {
                    RankPattern::Any
                } else {
                    RankPattern::Exact(axes.len() as u8)
                }
            }
            Some(ShapePattern::Any) | None => RankPattern::Any,
        }
    }
}
