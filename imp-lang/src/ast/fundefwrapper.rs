use super::*;

/// A wrapper around one or more overloaded function definitions with the same name.
/// During type inference, multiple definitions with the same name are grouped into
/// a single wrapper, enabling runtime dispatch (checked during code generation).
///
/// For now, we don't enforce non-overlapping signatures or validate overloads.
/// Future work: add static overload resolution, trait-based dispatch, etc.
#[derive(Clone, Debug)]
pub struct FundefWrapper<'ast, Ast: AstConfig> {
    pub name: String,
    pub overloads: Vec<Fundef<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> FundefWrapper<'ast, Ast> {
    pub fn new(name: String, overloads: Vec<Fundef<'ast, Ast>>) -> Self {
        FundefWrapper { name, overloads }
    }

    /// Create a single-overload wrapper (used before type inference).
    pub fn single(fundef: Fundef<'ast, Ast>) -> Self {
        let name = fundef.name.clone();
        FundefWrapper {
            name,
            overloads: vec![fundef],
        }
    }
}
