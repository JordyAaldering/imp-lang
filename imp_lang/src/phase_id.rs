use crate::Phase;

pub struct PhaseId {
    phase: Phase,
    id: usize,
}

impl PhaseId {
    pub fn new(phase: Phase) -> Self {
        Self { phase, id: 0 }
    }

    pub fn next(&mut self) -> String {
        self.id += 1;
        format!("{}{}", self.phase.name(), self.id)
    }
}
