use crate::Phase;

pub struct TravName {
    str: &'static str,
    id: usize,
}

impl TravName {
    pub fn new(phase: Phase) -> Self {
        use Phase::*;
        let str = match phase {
            RD => "rd",
            SCP => "scp",
            CTP => "ctp",
            ATP => "atp",
            FLT => "flt",
            SSA => "ssa",
            TI => "ti",
            DR => "dr",
            CF => "cf",
            DCR => "dcr",
            RNF => "rnf",
            CGC => "cgc",
            CGH => "cgh",
            CGRS => "cgrs",
        };
        Self { str, id: 0 }
    }

    pub fn next(&mut self) -> String {
        self.id += 1;
        format!("{}{}", self.str, self.id)
    }
}
