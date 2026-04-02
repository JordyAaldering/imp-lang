/// # Pre-processing (`pre`)
///
/// Prepare the parsed AST for safe internal use.

mod flatten;
mod to_ssa;

pub use flatten::flatten;
pub use to_ssa::to_ssa;
