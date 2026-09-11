//! # Type checking (`tc`)
mod validate_overloads;
mod sort_overloads;
mod type_infer;
mod resolve_dispatch;

pub use validate_overloads::validate_overloads;
pub use sort_overloads::sort_overloads;
pub use type_infer::type_infer;
pub use resolve_dispatch::resolve_dispatch;
