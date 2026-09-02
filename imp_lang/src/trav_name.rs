use crate::Phase;

pub struct TravName {
    str: &'static str,
    id: usize,
}

impl TravName {
    pub fn new(phase: Phase) -> Self {
        Self { str: phase.uid(), id: 0 }
    }

    pub fn next(&mut self) -> String {
        self.id += 1;
        format!("{}{}", self.str, self.id)
    }
}
