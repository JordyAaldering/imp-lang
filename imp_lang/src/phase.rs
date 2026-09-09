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
    /// Validate overloads
    VO,
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
    /// Print AST structure
    SHOW,
}

macro_rules! phase_log {
    ($($arg:tt)*) => {
        log::trace!(target: Self::PHASE.log_target(), $($arg)*);
    };
}

pub(crate) use phase_log;

impl Phase {
    /// A unique string identifier for the phase.
    pub const fn name(self) -> &'static str {
        match self {
            Self::RD => "rd",
            Self::SCP => "scp",
            Self::CTP => "ctp",
            Self::ATP => "atp",
            Self::FLT => "flt",
            Self::SSA => "ssa",
            Self::VO => "vo",
            Self::TI => "ti",
            Self::DR => "dr",
            Self::CF => "cf",
            Self::DCR => "dcr",
            Self::RNF => "rnf",
            Self::CGC => "cgc",
            Self::CGH => "cgh",
            Self::CGRS => "cgrs",
            Self::SHOW => "show",
        }
    }

    pub fn log_target(self) -> &'static str {
        match self {
            Self::RD => "imp_lang::phase::rd",
            Self::SCP => "imp_lang::phase::scp",
            Self::CTP => "imp_lang::phase::ctp",
            Self::ATP => "imp_lang::phase::atp",
            Self::FLT => "imp_lang::phase::flt",
            Self::SSA => "imp_lang::phase::ssa",
            Self::VO => "imp_lang::phase::vo",
            Self::TI => "imp_lang::phase::ti",
            Self::DR => "imp_lang::phase::dr",
            Self::CF => "imp_lang::phase::cf",
            Self::DCR => "imp_lang::phase::dcr",
            Self::RNF => "imp_lang::phase::rnf",
            Self::CGC => "imp_lang::phase::cgc",
            Self::CGH => "imp_lang::phase::cgh",
            Self::CGRS => "imp_lang::phase::cgrs",
            Self::SHOW => "imp_lang::phase::show",
        }
    }
}
