use std::fmt;

#[derive(Clone, Debug)]
pub struct Type {
    pub basetype: BaseType,
    pub shape: TypePattern,
}

#[derive(Clone, Debug)]
pub struct TypePattern(Vec<AxisPattern>);

#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// Variable-rank-and-shape capture: `d:shp`
    VariableRank {
        dim: String,
        shp: String,
    },
    /// Fixed-rank capture: `5:shp`
    FixedRank {
        dim: usize,
        shp: String,
    },
    /// Variable-length capture: `d`
    VariableLength {
        len: String,
    },
    /// Fixed-length capture: `5`
    FixedLength {
        len: usize,
    },
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
    ///
    /// (Not actually supported yet by the syntax or the compiler)
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

    pub const fn scalar(basetype: BaseType) -> Self {
        Self { basetype, shape: TypePattern::scalar() }
    }

    /// Check whether the type is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        self.shape.is_scalar()
    }

    /// Check whether the type is an array. Returns None if the rank is possibly non-zero, but variable.
    pub fn is_array(&self) -> Option<bool> {
        self.shape.is_array()
    }

    /// The minimum rank of this type. The actual rank may be higher if there is a variable-rank axis.
    pub fn min_rank(&self) -> usize {
        self.shape.min_rank()
    }

    /// The rank of this type, if it is fixed. Returns None if the rank is variable.
    pub fn rank(&self) -> Option<usize> {
        self.shape.rank()
    }
}

impl TypePattern {
    pub const fn scalar() -> Self {
        Self(Vec::new())
    }

    /// Check whether the type pattern is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        if self.0.is_empty() {
            Some(true)
        } else if self.min_rank() > 0 {
            // Definitely not a scalar
            Some(false)
        } else if self.0.iter().any(|axis| matches!(axis, AxisPattern::VariableRank { .. })) {
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
        } else if self.0.iter().any(|axis| matches!(axis, AxisPattern::VariableRank { .. })) {
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
            .map(|axis| axis.rank().unwrap_or(0))
            .sum()
    }

    /// The rank of this type pattern, if it is fixed. Returns None if the rank is variable.
    pub fn rank(&self) -> Option<usize> {
        self.0
            .iter()
            .try_fold(0, |acc, axis| {
                if let Some(dim) = axis.rank() {
                    Some(acc + dim)
                } else {
                    None
                }
            })
    }
}

impl AxisPattern {
    pub fn rank(&self) -> Option<usize> {
        match self {
            AxisPattern::VariableRank { .. } => None,
            AxisPattern::FixedRank { dim, .. } => Some(*dim),
            AxisPattern::VariableLength { .. } => Some(1),
            AxisPattern::FixedLength { .. } => Some(1),
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
            Self::VariableRank { dim, shp } => write!(f, "{dim}:{shp}"),
            Self::FixedRank { dim, shp } => write!(f, "{dim}:{shp}"),
            Self::VariableLength { len } => write!(f, "{len}"),
            Self::FixedLength { len } => write!(f, "{len}"),
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
