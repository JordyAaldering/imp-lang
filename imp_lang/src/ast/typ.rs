use std::fmt;

#[derive(Clone, Debug)]
pub struct Type {
    pub basetype: BaseType,
    pub shape: TypePattern,
}

#[derive(Clone, Debug)]
pub struct TypePattern(pub Vec<AxisPattern>);

#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// Shape capture, e.g.: `5:shp`, `d:shp`, or `_:_`.
    ShapePattern {
        dim: RankCapture,
        shp: Option<String>,
    },
    /// Rank capture, e.g.: `5`, `d`, or `_`.
    DimPattern {
        len: RankCapture,
    },
}

/// Captures the extent of a single dimension.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RankCapture {
    /// `5`: a dimension of fixed length 5.
    Fixed(usize),
    /// `d`: a dimension of variable length `d`.
    Var(String),
    /// `_`: a dimension of variable, unnamed length.
    Free,
}

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
    Udf(String),
}

impl Default for Type {
    fn default() -> Self {
        unreachable!("Type::default() only exists as a placeholder for traversals; it should never be called")
    }
}

impl Type {
    pub fn new(basetype: BaseType, axes: Vec<AxisPattern>) -> Self {
        Self { basetype, shape: TypePattern(axes) }
    }

    pub fn new_aud(basetype: BaseType) -> Self {
        Self { basetype, shape: TypePattern::aud() }
    }

    pub const fn scalar(basetype: BaseType) -> Self {
        Self { basetype, shape: TypePattern::scalar() }
    }

    pub fn aks_vector(basetype: BaseType, len: usize) -> Self {
        Self { basetype, shape: TypePattern::new(vec![AxisPattern::DimPattern { len: RankCapture::Fixed(len) }]) }
    }

    pub fn akd_vector(basetype: BaseType, len: RankCapture) -> Self {
        Self { basetype, shape: TypePattern::new(vec![AxisPattern::DimPattern { len }]) }
    }

    pub fn get_vector(&self) -> Option<&RankCapture> {
        match &self.shape.0[..] {
            [AxisPattern::DimPattern { len }] => Some(len),
            _ => None,
        }
    }

    pub fn type_pattern(&self) -> Option<&Vec<AxisPattern>> {
        if self.shape.0.is_empty() {
            None
        } else {
            Some(&self.shape.0)
        }
    }

    pub fn ctype(&self) -> String {
        if self.is_array().is_none_or(|x| x) {
            "ImpArrayRaw".to_string()
        } else {
            self.basetype.ctype()
        }
    }

    pub fn rstype(&self) -> String {
        if self.is_array().is_none_or(|x| x) {
            "ImpArrayRaw".to_string()
        } else {
            self.basetype.rstype()
        }
    }

    /// Check whether the type is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        self.shape.is_scalar()
    }

    pub fn is_definitely_scalar(&self) -> bool {
        self.is_scalar().unwrap_or(false)
    }

    pub fn is_maybe_scalar(&self) -> bool {
        self.is_scalar().unwrap_or(true)
    }

    /// Check whether the type is an array. Returns None if the rank is possibly non-zero, but variable.
    pub fn is_array(&self) -> Option<bool> {
        self.shape.is_array()
    }

    pub fn is_definitely_array(&self) -> bool {
        self.is_array().unwrap_or(false)
    }

    pub fn is_maybe_array(&self) -> bool {
        self.is_array().unwrap_or(true)
    }

    /// The rank of this type, if it is fixed. Returns None if the rank is variable.
    pub fn rank(&self) -> Option<usize> {
        self.shape.rank()
    }
}

impl TypePattern {
    pub fn new(axes: Vec<AxisPattern>) -> Self {
        debug_assert!(!axes.is_empty());
        Self(axes)
    }

    pub const fn scalar() -> Self {
        Self(Vec::new())
    }

    pub fn aud() -> Self {
        Self(vec![AxisPattern::ShapePattern { dim: RankCapture::Free, shp: None }])
    }

    /// Check whether the type pattern is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        if self.0.is_empty() {
            Some(true)
        } else if self.min_rank() > 0 {
            // Definitely not a scalar
            Some(false)
        } else if self.0.iter().any(|axis| matches!(axis, AxisPattern::ShapePattern { .. })) {
            // Minimum rank is zero, but there might be a variable-rank axis, so this could be a scalar or an array
            None
        } else {
            // Minimum rank is zero, and there are no variable-rank axes, so this is definitely a scalar
            Some(true)
        }
    }

    /// Check whether the type pattern is an array. Returns None if the rank is possibly non-zero, but variable.
    pub fn is_array(&self) -> Option<bool> {
        if self.0.is_empty() {
            Some(false)
        } else if self.min_rank() > 0 {
            // Definitely an array
            Some(true)
        } else if self.0.iter().any(|axis| matches!(axis, AxisPattern::ShapePattern { .. })) {
            // Minimum rank is zero, but there might be a variable-rank axis, so this could be a scalar or an array
            None
        } else {
            // Minimum rank is zero, and there are no variable-rank axes, so this is definitely a scalar
            Some(false)
        }
    }

    /// The minimum rank of this type pattern. The actual rank may be higher if there is a variable-rank axis.
    pub fn min_rank(&self) -> usize {
        self.0
            .iter()
            .map(|axis| {
                match axis.rank() {
                    RankCapture::Fixed(len) => *len,
                    _ => 0,
                }
            })
            .sum()
    }

    /// The rank of this type pattern, if it is fixed. Returns None if the rank is variable.
    pub fn rank(&self) -> Option<usize> {
        self.0
            .iter()
            .try_fold(0, |acc, axis| {
                match axis.rank() {
                    RankCapture::Fixed(len) => Some(acc + *len),
                    _ => None,
                }
            })
    }
}

impl AxisPattern {
    pub fn rank(&self) -> &RankCapture {
        match self {
            Self::ShapePattern { dim, .. } => dim,
            Self::DimPattern { len } => len,
        }
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

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shape.0.is_empty() {
            write!(f, "{}", self.basetype)
        } else {
            write!(f, "{}[{}]", self.basetype, self.shape)
        }
    }
}

impl fmt::Display for TypePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .map(|axis| axis.to_string())
            .collect::<Vec<_>>()
            .join(",")
            .fmt(f)
    }
}

impl fmt::Display for AxisPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShapePattern { dim, shp } => {
                let shp = shp.as_ref().map(|s| s.as_str()).unwrap_or("_");
                write!(f, "{dim}:{shp}")
            }
            Self::DimPattern { len } => write!(f, "{len}"),
        }
    }
}

impl fmt::Display for RankCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(len) => write!(f, "{len}"),
            Self::Var(len) => write!(f, "{len}"),
            Self::Free => write!(f, "_"),
        }
    }
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool => write!(f, "bool"),
            Self::Usize => write!(f, "usize"),
            Self::U32 => write!(f, "u32"),
            Self::U64 => write!(f, "u64"),
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::Udf(udf) => write!(f, "{udf}"),
        }
    }
}
