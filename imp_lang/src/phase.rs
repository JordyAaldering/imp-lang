#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(clap::ValueEnum)]
pub enum Phase {
    /// Read input
    RD,
    /// Scanning/parsing
    SCP,
    /// Check type pattern
    CTP,
    /// Analyse type pattern
    ATP,
    /// Flatten
    FLT,
    /// Convert to SSA
    SSA,
    /// Type inference
    TI,
    /// Function dispatch resolution
    DR,
    /// Constant folding
    CF,
    /// Dead code removal
    DCR,
    /// Rename fundefs
    RNF,
    /// C header code generation
    CGH,
    /// C code generation
    CGC,
    /// Rust FFI code generation
    CGRS,
}

impl Phase {
    /// A unique string identifier for the phase.
    pub fn uid(self) -> &'static str {
        match self {
            Self::RD => "rd",
            Self::SCP => "scp",
            Self::CTP => "ctp",
            Self::ATP => "atp",
            Self::FLT => "flt",
            Self::SSA => "ssa",
            Self::TI => "ti",
            Self::DR => "dr",
            Self::CF => "cf",
            Self::DCR => "dcr",
            Self::RNF => "rnf",
            Self::CGC => "cgc",
            Self::CGH => "cgh",
            Self::CGRS => "cgrs",
        }
    }
}
