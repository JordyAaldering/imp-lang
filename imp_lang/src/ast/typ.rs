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
    pub basetype: BaseType,
    pub shape: TypePattern,
}

impl Default for Type {
    fn default() -> Self {
        unreachable!("Type::default() only exists as a placeholder for traversals; it should never be called")
    }
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
    Dim(DimCapture),
    /// Rank-and-shape capture (`d:shp`): binds the full rank and shape of the array
    Rank(RankCapture),
}

/// A single dimension pattern entry
#[derive(Clone, Debug)]
pub enum DimCapture {
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
    pub dim: DimCapture,
    /// Name bound to the array's shape vector (`arr.shp`) as a `usize[d]` array
    pub shp: String,
}

impl Type {
    pub const fn scalar(basetype: BaseType) -> Self {
        Self { basetype, shape: TypePattern::Scalar }
    }

    pub fn vector_dim(basetype: BaseType, dim: DimCapture) -> Self {
        Self { basetype, shape: TypePattern::Axes(vec![AxisPattern::Dim(dim)]) }
    }

    /// TODO: we might not be sure whether this is a scalar (i32[d:shp] can be both)
    pub fn is_scalar(&self) -> bool {
        matches!(self.shape, TypePattern::Scalar)
    }

    /// TODO: we might not be sure whether this is an array (i32[d:shp] can be both)
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
        }
    }
}

impl TypePattern {
    /// TODO: this is not yet correct, currently it defines any one-dimensional array
    /// But first, lets make the rust type checker happy
    pub fn any() -> Self {
        TypePattern::Axes(vec![AxisPattern::Rank(RankCapture {
            dim: DimCapture::any(),
            shp: String::new(),
        })])
    }
}

impl DimCapture {
    pub fn any() -> Self {
        DimCapture::Var(String::new())
    }
}

impl BaseType {
    /// Rust type name for this base type.
    pub fn rstype(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::Usize => "usize".to_string(),
            Self::U32 => "u32".to_string(),
            Self::U64 => "u64".to_string(),
            Self::I32 => "i32".to_string(),
            Self::I64 => "i64".to_string(),
            Self::F32 => "f32".to_string(),
            Self::F64 => "f64".to_string(),
            Self::Udf(udf) => udf.clone(),
        }
    }

    /// C type name for this base type.
    pub fn ctype(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::Usize => "size_t".to_string(),
            Self::U32 => "uint32_t".to_string(),
            Self::U64 => "uint64_t".to_string(),
            Self::I32 => "int32_t".to_string(),
            Self::I64 => "int64_t".to_string(),
            Self::F32 => "float".to_string(),
            Self::F64 => "double".to_string(),
            Self::Udf(udf) => udf.clone(),
        }
    }
}
