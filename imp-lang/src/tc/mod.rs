//! # Type checking (`tc`)
mod resolve_dispatch;
mod type_infer;

pub use resolve_dispatch::resolve_dispatch;
pub use type_infer::type_infer;
