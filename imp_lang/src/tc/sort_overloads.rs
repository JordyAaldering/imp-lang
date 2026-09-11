//! Overloads are sorted by how `precisely' they are defined. For example, int[5] is more
//! precise than int[.], which is more precise than int[*].
//!
//! When a function has multiple arguments, two overloads can be fully disjoint. For example;
//!
//! ```imp
//! foo(x: int, y: int[+]) -> int
//!
//! foo(x: int[+], y: int) -> int
//! ```
//!
//! Here, the `x` argument of the first instance is more precise, but in `y` the second
//! instance is more precise. In our mental model, this results in a directed acyclic graph (DAG).
//! However, we don't have to implement an DAG, we can just sort overloads as a flat list, with
//! an arbitrary order for such disjoint overloads.
//!
//! When finding which overload to use, we iterate through this sorted list in order, and then
//! the first match we find is also the most precise match. When two overloads are fully disjoint,
//! it can never be that both match, so it doesn't matter in which way we order them.

use crate::{Phase, ast::*};

pub fn sort_overloads<'ast>(program: &mut Program<'ast, UntypedAst>) -> Result<(), String> {
    let mut trav = SortOverloads::default();
    trav.trav_program(program);
    if trav.errors.is_empty() {
        Ok(())
    } else {
        Err(trav.errors.join("\n"))
    }
}

#[derive(Default)]
struct SortOverloads {
    errors: Vec<String>,
}

impl<'ast> Traverse<'ast> for SortOverloads {
    const PHASE: Phase = Phase::SO;

    type Ast = UntypedAst;

    type DeclOut = ();

    type ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        todo!()
    }
}
