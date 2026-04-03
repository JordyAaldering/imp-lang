//! # Type patterns (`tp`)
//! Prepare the parsed type patterns for safe internal use.
mod analyse_tp;
mod check_tp;
mod generate_tp_constraints;

pub use analyse_tp::analyse_tp;
//pub use check_tp::check_tp;
//pub use generate_tp_constraints::generate_tp_constraints;