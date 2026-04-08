#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseType {
    I32,
    I64,
    U32,
    U64,
    Usize,
    F32,
    F64,
    Bool,
    /// User-defined type
    ///
    /// (Not actually supported yet by the syntax or the compiler)
    Udf(String),
}

/// A fully resolved type: element type, shape pattern, and compile-time knowledge
#[derive(Clone, Debug)]
pub struct Type {
    pub ty: BaseType,
    /// Shape as declared in the source pattern
    pub shape: TypePattern,
    // Compile-time knowledge about this shape, derived by `tp::analyse_tp`
    //pub knowledge: TypeKnowledge,
}

/// The shape component of a type pattern.
#[derive(Clone, Debug)]
pub enum TypePattern {
    /// Rank-0; no array dimensions
    ///
    /// Example: `u32`
    Scalar,
    /// Explicit list of dimension and rest patterns.
    ///
    /// Example: `u32[42]`, `u32[n]`, `u32[m,..rest,n]`
    Axes(Vec<AxisPattern>),
    /// Shape fully unconstrained.
    ///
    /// Example: `u32[*]`
    Any,
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
    Var(ExtentVar),
}

/// A named dimension symbol with its binding role
#[derive(Clone, Debug)]
pub struct ExtentVar {
    pub name: String,
    pub role: SymbolRole,
}

/// A `d:shp` rank capture — binds the rank scalar (`d`) and the shape vector (`shp`) from
/// the runtime array descriptor, without constraining the rank at compile time
#[derive(Clone, Debug)]
pub struct RankCapture {
    /// Name bound to the array's rank (`arr.dim`) as a `usize` scalar
    pub dim_name: String,
    /// Name bound to the array's shape vector (`arr.shp`) as a `usize[d]` array
    pub shp_name: String,
    /// Whether `dim_name` is being introduced (Define) or must equal a prior symbol
    pub dim_role: SymbolRole,
}

/// Whether a symbol is first introduced here or constrained to equal a prior definition
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolRole {
    /// First occurrence, this site defines the symbol's value
    Define,
    /// Subsequent occurrence, must equal the defining occurrence
    Use,
}

impl Type {
    pub fn scalar(ty: BaseType) -> Self {
        Self { ty, shape: TypePattern::Scalar }
    }

    pub fn vector(ty: BaseType, extent: &str) -> Self {
        let dim = if extent == "." {
            DimPattern::Any
        } else {
            DimPattern::Var(ExtentVar { name: extent.to_owned(), role: SymbolRole::Use })
        };
        Self::vector_dim(ty, dim)
    }

    pub fn vector_dim(ty: BaseType, dim: DimPattern) -> Self {
        Self { ty, shape: TypePattern::Axes(vec![AxisPattern::Dim(dim)]) }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.shape, TypePattern::Scalar)
    }

    pub fn is_array(&self) -> bool {
        !self.is_scalar()
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
            TypePattern::Any => None,
        }
    }
}
