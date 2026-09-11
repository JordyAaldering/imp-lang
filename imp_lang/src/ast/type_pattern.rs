use std::{cmp, fmt};

use crate::ast::ShapeKnowledge;

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

    pub fn is_definitely_scalar(&self) -> bool {
        self.is_scalar().unwrap_or(false)
    }

    pub fn is_maybe_scalar(&self) -> bool {
        self.is_scalar().unwrap_or(true)
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

    pub fn is_definitely_array(&self) -> bool {
        self.is_array().unwrap_or(false)
    }

    pub fn is_maybe_array(&self) -> bool {
        self.is_array().unwrap_or(true)
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

    /// The shape of this type pattern, if it is fixed. Returns None if any shape component is variable.
    pub fn shape(&self) -> Option<Vec<usize>> {
        self.0
            .iter()
            .map(|axis| {
                match axis.rank() {
                    RankCapture::Fixed(len) => Some(*len),
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
                match axis.rank() {
                    RankCapture::Fixed(len) => Some(acc + *len),
                    _ => None,
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
    pub fn rank(&self) -> &RankCapture {
        match self {
            Self::ShapePattern { dim, .. } => dim,
            Self::DimPattern { len } => len,
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

// impl cmp::PartialOrd for RankCapture {
//     /// Defines which of two rank captures is more 'precise'. A fixed rank is more
//     /// precise than a variable rank, which is more precise than a free rank. Returns
//     /// `None` if both ranks are equally precise, even if their actual lengths differ.
//     fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
//         match (self, other) {
//             // `self` is more precise than `other`
//             (Self::Fixed(_), Self::Var(_)) |
//             (Self::Fixed(_), Self::Free) |
//             (Self::Var(_), Self::Free) =>
//                 Some(cmp::Ordering::Less),
//             // `other` is more precise than `self`
//             (Self::Var(_), Self::Fixed(_)) |
//             (Self::Free, Self::Fixed(_)) |
//             (Self::Free, Self::Var(_)) =>
//                 Some(cmp::Ordering::Greater),
//             // `self` and `other` are equally precise (but possibly different lengths)
//             (Self::Fixed(_), Self::Fixed(_)) |
//             (Self::Var(_), Self::Var(_)) |
//             (Self::Free, Self::Free) =>
//                 None,
//         }
//     }
// }

impl fmt::Display for RankCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(len) => write!(f, "{len}"),
            Self::Var(len) => write!(f, "{len}"),
            Self::Free => write!(f, "_"),
        }
    }
}
