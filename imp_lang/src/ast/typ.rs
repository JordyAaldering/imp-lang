use std::fmt;

use super::type_pattern::*;

#[derive(Clone, Debug)]
pub struct Type {
    pub basetype: BaseType,
    pub shape: TypePattern,
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

    pub fn get_vector(&self) -> Option<&AxisPattern> {
        match &self.shape.0[..] {
            [AxisPattern::FixedDim { .. }] |
            [AxisPattern::VarDim { .. }] |
            [AxisPattern::FixedShape { dim: 1, .. }] =>
                Some(&self.shape.0[0]),
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

    pub fn c_str(&self) -> String {
        if self.is_array().is_none_or(|x| x) {
            "ImpArrayRaw".to_string()
        } else {
            self.basetype.c_str()
        }
    }

    pub fn rs_str(&self) -> String {
        if self.is_array().is_none_or(|x| x) {
            "ImpArrayRaw".to_string()
        } else {
            self.basetype.rs_str()
        }
    }

    /// Check whether the type is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        self.shape.is_scalar()
    }

    pub fn is_definitely_scalar(&self) -> bool {
        self.is_scalar().unwrap_or(false)
    }

    // pub fn is_maybe_scalar(&self) -> bool {
    //     self.is_scalar().unwrap_or(true)
    // }

    /// Check whether the type is an array. Returns None if the rank is possibly non-zero, but variable.
    pub fn is_array(&self) -> Option<bool> {
        self.shape.is_array()
    }

    // pub fn is_definitely_array(&self) -> bool {
    //     self.is_array().unwrap_or(false)
    // }

    pub fn is_maybe_array(&self) -> bool {
        self.is_array().unwrap_or(true)
    }

    /// The rank of this type, if it is fixed. Returns None if the rank is variable.
    pub fn rank(&self) -> Option<usize> {
        self.shape.rank()
    }
}

impl BaseType {
    /// Rust type name for this base type.
    pub fn rs_str(&self) -> String {
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
    pub fn c_str(&self) -> String {
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
