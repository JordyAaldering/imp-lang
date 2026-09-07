pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn try_left(self) -> Option<L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => None,
        }
    }

    pub fn try_right(self) -> Option<R> {
        match self {
            Either::Left(_) => None,
            Either::Right(r) => Some(r),
        }
    }

    pub fn left_or(self, default: L) -> L {
        match self {
            Either::Left(l) => l,
            Either::Right(_) => default,
        }
    }

    pub fn right_or(self, default: R) -> R {
        match self {
            Either::Left(_) => default,
            Either::Right(r) => r,
        }
    }
}
