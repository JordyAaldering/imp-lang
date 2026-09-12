//! # Type patterns (`tp`)
//!
//! Prepare the parsed type patterns for internal use.
mod analyse_tp;
mod validate_tp;

pub use analyse_tp::analyse_tp;
pub use validate_tp::validate_tp;
