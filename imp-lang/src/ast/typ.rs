#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BaseType {
    Bool,
    Usize,
    U32,
    U64,
    I32,
    I64,
    F32,
    F64,
    /// User-defined type
    ///
    /// (Not actually supported yet by the syntax or the compiler)
    Udf(String),
}

#[derive(Clone, Debug)]
pub struct Type {
    pub ty: BaseType,
    pub shape: TypePattern,
}

#[derive(Clone, Debug)]
pub enum TypePattern {
    /// Rank-0; no array dimensions
    ///
    /// Example: `u32`
    Scalar,
    /// Explicit list of dimension and rest patterns.
    ///
    /// Example: `u32[42]`, `u32[n]`, `u32[m,d:shp,n]`
    Axes(Vec<AxisPattern>),
}

/// One entry in an `Axes` shape pattern
#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// A single dimension (`_`, `42`, or a named symbol)
    Dim(DimPattern),
    /// Rank-and-shape capture (`d:shp`): binds the full rank and shape of the array
    Rank(RankCapture),
}

/// A single dimension pattern entry
#[derive(Clone, Debug)]
pub enum DimPattern {
    /// Size unknown
    ///
    /// Example: `u32[_]`
    Any,
    /// Compile-time constant.
    ///
    /// Example: `u32[42]`
    Known(usize),
    /// Named symbol
    ///
    /// Example: `u32[n]`, `u32[len]`
    Var(String),
}

/// A `d:shp` rank capture — binds the rank scalar (`d`) and the shape vector (`shp`) from
/// the runtime array descriptor, without constraining the rank at compile time
#[derive(Clone, Debug)]
pub struct RankCapture {
    /// Name bound to the array's rank (`arr.dim`) as a `usize` scalar
    pub dim_name: String,
    /// Name bound to the array's shape vector (`arr.shp`) as a `usize[d]` array
    pub shp_name: String,
}

impl Type {
    pub const fn scalar(ty: BaseType) -> Self {
        Self { ty, shape: TypePattern::Scalar }
    }

    pub fn vector_dim(ty: BaseType, dim: DimPattern) -> Self {
        Self { ty, shape: TypePattern::Axes(vec![AxisPattern::Dim(dim)]) }
    }

    /// TODO: we might not be sure whether this is a scalar (i32[d:shp] can be both)
    pub fn is_scalar(&self) -> bool {
        matches!(self.shape, TypePattern::Scalar)
    }

    /// TODO: we might not be sure whether this is an array (i32[d:shp] can be both)
    pub fn is_array(&self) -> bool {
        !self.is_scalar()
    }

    /// TODO: merge this function and the ones above it into an enum result
    pub fn is_array_or_scalar(&self) -> bool {
        false
    }

    pub fn rank(&self) -> Option<u8> {
        match &self.shape {
            TypePattern::Scalar => Some(0),
            TypePattern::Axes(axes) => {
                if axes.iter().any(|a| matches!(a, AxisPattern::Rank(_))) {
                    None
                } else {
                    Some(axes.len() as u8)
                }
            }
        }
    }
}

impl TypePattern {
    /// TODO: this is not yet correct, currently it defines any one-dimensional array
    /// But first, lets make the rust type checker happy
    pub fn any() -> Self {
        TypePattern::Axes(vec![AxisPattern::Dim(DimPattern::Any)])
    }
}