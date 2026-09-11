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
//!
//! TODO: The current implementation is only a mock implementation. It only works for statically
//! known differences in rank, and doesn't take into account the shape of the arguments.

use std::cmp::Ordering;

use crate::{Phase, ast::*, phase::phase_log};

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
        let fundefs = &program.fundefs;

        for (name, signature, ids) in program.overloads.flatten_mut() {
            phase_log!("Sorting overloads for `{}` with signature {}", name, signature);
            ids.sort_by(|a, b| compare(a, b, fundefs));
        }
    }
}

/// Less if at least one argument of `a` is more precise than the corresponding argument of `b`,
/// and no argument of `b` is more precise than the corresponding argument of `a`.
///
/// Greater if at least one argument of `b` is more precise than the corresponding argument of `a`,
/// and no argument of `a` is more precise than the corresponding argument of `b`.
///
/// Equal if `a` and `b` are disjoint.
///
/// TODO: currently panics on invalid definitions. Instead a human-readable error should be shown.
fn compare<'ast>(
    a: &id_arena::Id<Fundef<'ast, UntypedAst>>,
    b: &id_arena::Id<Fundef<'ast, UntypedAst>>,
    fundefs: &id_arena::Arena<Fundef<'ast, UntypedAst>>,
) -> Ordering {

    let a_args = &fundefs[*a].args;
    let b_args = &fundefs[*b].args;

    debug_assert_eq!(a_args.len(), b_args.len());

    let mut ord = None;

    for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
        let a_shape = &a_arg.ty.shape;
        let b_shape = &b_arg.ty.shape;

        if (a_shape.min_rank() < b_shape.min_rank()) ||
            (a_shape.is_definitely_scalar() && b_shape.is_maybe_scalar()) ||
            (a_shape.is_maybe_array() && b_shape.is_definitely_array())
        {
            // `a` is more precise than `b` in this argument
            match ord {
                Some(Ordering::Greater) => {
                    // A previous argument was more precise in the other direction, so these two overloads are disjoint.
                    ord = Some(Ordering::Equal)
                }
                Some(Ordering::Less) => {
                    // The function is already more precise in this direction, so we keep it like that.
                }
                Some(Ordering::Equal) => {
                    // The functions are disjoint, so we keep it like that.
                }
                None => {
                    // There was no ordering yet, so we define it now.
                    ord = Some(Ordering::Less)
                }
            }

        } else if (b_shape.min_rank() < a_shape.min_rank()) ||
            (b_shape.is_definitely_scalar() && a_shape.is_maybe_scalar()) ||
            (b_shape.is_maybe_array() && a_shape.is_definitely_array())
        {
            // `b` is more precise than `a` in this argument
            match ord {
                Some(Ordering::Greater) => {
                    // The function is already more precise in this direction, so we keep it like that.
                }
                Some(Ordering::Less) => {
                    // A previous argument was more precise in the other direction, so these two overloads are disjoint.
                    ord = Some(Ordering::Equal)
                }
                Some(Ordering::Equal) => {
                    // The functions are disjoint, so we keep it like that.
                }
                None => {
                    // There was no ordering yet, so we define it now.
                    ord = Some(Ordering::Greater)
                }
            }

        } else {
            // The arguments are 'equally precise', so we don't change the ordering.
        }
    }

    ord.expect(&format!("No clear ordering between `{}({})` and `{}({})`",
        fundefs[*a].name, fundefs[*a].args.iter().map(|arg| arg.ty.to_string()).collect::<Vec<_>>().join(", "),
        fundefs[*b].name, fundefs[*b].args.iter().map(|arg| arg.ty.to_string()).collect::<Vec<_>>().join(", "),
    ))
}
