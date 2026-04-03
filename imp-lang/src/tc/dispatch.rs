/// At this point, function calls point to the corresponding wrappers.
///
/// If possible, we want to point to the specific overload instead.
///
/// The simplest case is when the wrapper has only one overload, in which case we can directly point to it.
///
/// If all overloads have unique base type/argument count combinations, this is also straightforward.
///
/// The main difficulty is when multiple overloads differ only in their type patterns.
#[allow(dead_code)]
pub struct Dispatch {}
