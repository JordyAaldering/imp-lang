#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct Span {
    from_line: usize,
    from_col: usize,
    to_line: usize,
    to_col: usize,
}

impl Span {
    pub fn new(line: usize, from: usize, to: usize) -> Self {
        Self {
            from_line: line,
            from_col: from,
            to_line: line,
            to_col: to,
        }
    }

    pub fn extend(&mut self, other: &Span) {
        debug_assert!(self.from_line <= other.to_line);
        debug_assert!(self.from_col <= other.to_col);
        self.to_line = other.to_line;
        self.to_col = other.to_col;
    }

    pub fn to(&self, other: &Span) -> Self {
        debug_assert!(self.from_line <= other.to_line);
        debug_assert!(self.from_col <= other.to_col);
        Self {
            from_line: self.from_line,
            from_col: self.from_col,
            to_line: other.to_line,
            to_col: other.to_col,
        }
    }
}
