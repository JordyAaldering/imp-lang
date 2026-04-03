use super::*;

/// The resolved dispatch target of a typed function call.
///
/// After type inference, every call knows at minimum which wrapper it targets.
/// If a specific overload was statically selected (e.g. by overload resolution),
/// the `Overload` variant holds the exact definition.
///
/// Example:
/// - `foo(x)` where `foo` has one overload → `Wrapper(&wrapper_foo)`
/// - After static resolution: `Overload(&fundef_foo_usize)`
#[derive(Clone, Debug)]
pub enum CallTarget<'ast> {
    /// Target is the full wrapper; exact overload resolved at codegen / runtime.
    Wrapper(&'ast FundefWrapper<'ast, TypedAst>),
    /// Target is a statically-known specific overload.
    Overload(&'ast Fundef<'ast, TypedAst>),
}

impl<'ast> CallTarget<'ast> {
    pub fn name(&self) -> &str {
        match self {
            CallTarget::Wrapper(w) => &w.name,
            CallTarget::Overload(f) => &f.name,
        }
    }

    pub fn wrapper(&self) -> Option<&'ast FundefWrapper<'ast, TypedAst>> {
        match self {
            CallTarget::Wrapper(w) => Some(w),
            CallTarget::Overload(_) => None,
        }
    }

    pub fn overload(&self) -> Option<&'ast Fundef<'ast, TypedAst>> {
        match self {
            CallTarget::Wrapper(_) => None,
            CallTarget::Overload(f) => Some(f),
        }
    }
}
