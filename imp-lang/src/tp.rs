//! # Type patterns (`tp`)
//!
//! Prepare the parsed type patterns for internal use.
mod analyse_tp;
mod check_tp;

pub use analyse_tp::analyse_tp;
pub use check_tp::check_tp;
