#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShapeKnowledge {
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
