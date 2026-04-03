/// The base scalar element type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseType {
    U32,
    Usize,
    Bool,
}

/// A fully resolved type: element type, shape pattern, and compile-time knowledge.
#[derive(Clone, Debug)]
pub struct Type {
    pub ty: BaseType,
    /// Shape as declared in the source pattern.
    pub shape: ShapePattern,
    /// Compile-time knowledge about this shape, derived by `tp::analyse_tp`.
    pub knowledge: TypeKnowledge,
}

/// The shape component of a type pattern.
#[derive(Clone, Debug)]
pub enum ShapePattern {
    /// Rank-0; no array dimensions.
    /// Surface example: `u32`.
    Scalar,
    /// Explicit list of dimension and rest patterns.
    /// Surface examples: `u32[42]`, `u32[n]`, `u32[m,..rest,n]`.
    Axes(Vec<AxisPattern>),
    /// Shape fully unconstrained.
    /// Surface example: `u32[*]`.
    Any,
}

/// One entry in an `Axes` shape pattern.
#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// A single dimension (`_`, `42`, or a named symbol).
    Dim(DimPattern),
    /// Rank-and-shape capture (`d:shp`): binds the full rank and shape of the array.
    Rank(RankCapture),
}

/// A single dimension pattern entry.
#[derive(Clone, Debug)]
pub enum DimPattern {
    /// Size unknown. Surface example: `u32[_]`.
    Any,
    /// Compile-time constant. Surface example: `u32[42]`.
    Known(u64),
    /// Named symbol. Surface examples: `u32[n]`, `u32[len]`.
    Var(ExtentVar),
}

/// A named dimension symbol with its binding role.
#[derive(Clone, Debug)]
pub struct ExtentVar {
    pub name: String,
    pub role: SymbolRole,
}

/// A `d:shp` rank capture — binds the rank scalar (`d`) and the shape vector (`shp`) from
/// the runtime array descriptor, without constraining the rank at compile time.
#[derive(Clone, Debug)]
pub struct RankCapture {
    /// Name bound to the array's rank (`arr.dim`) as a `usize` scalar.
    pub dim_name: String,
    /// Name bound to the array's shape vector (`arr.shp`) as a `usize[d]` array.
    pub shp_name: String,
    /// Whether `dim_name` is being introduced (Define) or must equal a prior symbol (Use).
    pub dim_role: SymbolRole,
}

/// Whether a symbol is first introduced here or constrained to equal a prior definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolRole {
    /// First occurrence — this site defines the symbol's value.
    Define,
    /// Subsequent occurrence — must equal the defining occurrence.
    Use,
}

/// SaC-inspired compile-time knowledge classes for arrays, derived by `tp::analyse_tp`.
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub enum TypeKnowledge {
    /// Rank-0 scalar; not an array.
    Scalar,
    /// Array Known Shape: rank and all symbolic extents are statically constrained.
    /// Example: `u32[n]`, `u32[42,m]`.
    AKS,
    /// Array Known Dimension: rank is known but at least one extent is unconstrained (`_`).
    /// Example: `u32[_]`, `u32[n,_]`.
    AKD,
    /// Array Unknown Dimension: shape fully unconstrained.
    /// Example: `u32[*]`.
    AUD,
    /// Array Unknown Dimension Greater than N: a `..rest` capture is present.
    /// Example: `u32[m,n,..rest]` gives `AUDGN { min_rank: 2 }`.
    AUDGN { min_rank: u8 },
}

impl Type {
    pub fn scalar(ty: BaseType) -> Self {
        Self { ty, shape: ShapePattern::Scalar, knowledge: TypeKnowledge::Scalar }
    }

    pub fn vector(ty: BaseType, extent: &str) -> Self {
        let dim = if extent == "." {
            DimPattern::Any
        } else {
            DimPattern::Var(ExtentVar { name: extent.to_owned(), role: SymbolRole::Use })
        };
        Self::vector_dim(ty, dim)
    }

    /// Rank-1 type with the given single dimension pattern.
    pub fn vector_dim(ty: BaseType, dim: DimPattern) -> Self {
        Self { ty, shape: ShapePattern::Axes(vec![AxisPattern::Dim(dim)]), knowledge: TypeKnowledge::AKS }
    }

    pub fn with_knowledge(mut self, knowledge: TypeKnowledge) -> Self {
        self.knowledge = knowledge;
        self
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.shape, ShapePattern::Scalar)
    }

    pub fn is_array(&self) -> bool {
        !self.is_scalar()
    }

    /// Returns the exact rank if statically known, `None` when a `d:shp` capture or `Any` is present.
    pub fn rank(&self) -> Option<u8> {
        match &self.shape {
            ShapePattern::Scalar => Some(0),
            ShapePattern::Axes(axes) => {
                if axes.iter().any(|a| matches!(a, AxisPattern::Rank(_))) {
                    None
                } else {
                    Some(axes.len() as u8)
                }
            }
            ShapePattern::Any => None,
        }
    }

    pub fn is_vector(&self) -> bool {
        self.rank() == Some(1)
    }
}
