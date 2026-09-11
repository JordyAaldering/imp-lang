use std::fmt;

use crate::ast::ShapeKnowledge;

#[derive(Clone, Debug)]
pub struct TypePattern(pub Vec<AxisPattern>);

/// TODO: using RankCapture is really messy. They have a different meaning in `dim` and `len`.
/// RankCapture should at some point be removed.
#[derive(Clone, Debug)]
pub enum AxisPattern {
    /// Constant-rank capture, e.g. `5`.
    FixedDim {
        len: usize,
    },
    /// Variable-rank capture, e.g. `5`.
    VarDim {
        len: Option<String>,
    },
    /// Fixed-length shape capture, e.g. `5:shp`.
    FixedShape {
        dim: usize,
        shp: Option<String>,
    },
    /// Variable-length shape capture, e.g. `d>0:shp`.
    VarShape {
        dim: Option<String>,
        min_dim: usize,
        shp: Option<String>,
    },
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
        Self(vec![AxisPattern::VarShape { dim: None, min_dim: 0, shp: None }])
    }

    /// Check whether the type pattern is scalar. Returns None if the rank is possibly zero, but variable.
    pub fn is_scalar(&self) -> Option<bool> {
        if self.0.is_empty() {
            Some(true)
        } else if self.min_rank() > 0 {
            // Definitely not a scalar
            Some(false)
        } else if self.rank().is_none() {
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
        } else if self.rank().is_none() {
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

    /// The shape of this type pattern, if it is fixed. Returns None if any shape component is variable.
    pub fn shape(&self) -> Option<Vec<usize>> {
        self.0
            .iter()
            .map(|axis| {
                match axis {
                    AxisPattern::FixedDim { len } => Some(*len),
                    _ => None,
                }
            })
            .collect()
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

    pub fn shape_knowledge(&self) -> ShapeKnowledge {
        if let Some(shape) = self.shape() {
            ShapeKnowledge::AKS(shape)
        } else if let Some(rank) = self.rank() {
            ShapeKnowledge::AKD(rank)
        } else {
            let min_rank = self.min_rank();
            if min_rank > 0 {
                ShapeKnowledge::AUDGN(min_rank)
            } else {
                ShapeKnowledge::AUD
            }
        }
    }
}

impl AxisPattern {
    pub fn rank(&self) -> Option<usize> {
        match self {
            Self::FixedDim { .. } | Self::VarDim { .. } => Some(1),
            Self::FixedShape { dim, .. } => Some(*dim),
            Self::VarShape { .. } => None,
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
            Self::FixedDim { len } =>
                write!(f, "{}", len),
            Self::VarDim { len } =>
                write!(f, "{}", len.clone().unwrap_or("_".to_string())),
            Self::FixedShape { dim, shp } =>
                write!(f, "{}:{}", dim, shp.clone().unwrap_or("_".to_string())),
            Self::VarShape { dim, min_dim, shp } => {
                if *min_dim == 0 {
                    write!(f, "{}:{}", dim.clone().unwrap_or("_".to_string()), shp.clone().unwrap_or("_".to_string()))
                } else {
                    write!(f, "{}>{}:{}", dim.clone().unwrap_or("_".to_string()), min_dim, shp.clone().unwrap_or("_".to_string()))
                }
            }
        }
    }
}
