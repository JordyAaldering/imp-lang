/// Will be used to determine break points for testing and debugging.
///
/// In the future, every phase should probably implement a trait,
/// that way we can have a single `run_phase`, which takes care of breaking,
/// but also profiling like memory usage and time taken.
//#[derive(clap::ValueEnum)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Phase {
    // Scanning-parsing
    ScpParse,
    // Type patterns
    TpAnalyse,
    TpCheck,
    // Pre-processing
    PreFlatten,
    PreSsa,
    // Type checking
    TcTypeInfer,
    TcCheckWrappers,
    TcDispatch,
    // Optimisation cycle
    OptConstantFold,
    OptDeadCodeRemoval,
    // Code generation
    CgEmitFfi,
    CgEmitC,
    CgEmitH,
}
