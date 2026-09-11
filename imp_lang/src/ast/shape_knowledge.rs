#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShapeKnowledge {
    #[allow(unused)]
    /// Known value
    AKV,
    /// Known shape
    AKS(Vec<usize>),
    /// Known rank
    AKD(usize),
    /// Unknown rank >N
    AUDGN(usize),
    /// Unknown rank
    AUD,
}

#[cfg(test)]
mod tests {
    use super::*;
    use parameterized::parameterized;

    #[parameterized(
        pair = {
            (ShapeKnowledge::AKV, ShapeKnowledge::AKS(vec![1, 2])),
            (ShapeKnowledge::AKV, ShapeKnowledge::AKD(3)),
            (ShapeKnowledge::AKV, ShapeKnowledge::AUDGN(2)),
            (ShapeKnowledge::AKV, ShapeKnowledge::AUD),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AKS(vec![1, 2, 3])),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AKS(vec![1, 3])),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AKS(vec![3, 2])),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AKD(3)),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AUDGN(2)),
            (ShapeKnowledge::AKS(vec![1, 2]), ShapeKnowledge::AUD),
            (ShapeKnowledge::AKD(3), ShapeKnowledge::AKD(4)),
            (ShapeKnowledge::AKD(3), ShapeKnowledge::AUDGN(2)),
            (ShapeKnowledge::AKD(3), ShapeKnowledge::AUD),
            (ShapeKnowledge::AUDGN(2), ShapeKnowledge::AUDGN(3)),
            (ShapeKnowledge::AUDGN(2), ShapeKnowledge::AUD),
        }
    )]
    fn test_shape_knowledge_a_lt_b(pair: (ShapeKnowledge, ShapeKnowledge)) {
        let (l, r) = pair;
        assert!(l < r)
    }
}
