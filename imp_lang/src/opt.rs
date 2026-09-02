//! # Optimisation cycle (`opt`)
mod constant_fold;
mod dead_code_removal;

pub use constant_fold::constant_fold;
pub use dead_code_removal::dead_code_removal;
