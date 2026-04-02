#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct Span {
    line: usize,
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(line: usize, start: usize, end: usize) -> Self {
        Self { line, start, end }
    }
}
