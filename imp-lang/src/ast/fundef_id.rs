use id_arena::Id;

use super::*;

/// A lightweight, `Copy` handle to a `Fundef` stored in `Program.fundefs`.
///
/// Backed by `id_arena::Id`, which is generic over the pointee type: `FundefId<'ast, ParsedAst>`,
/// `FundefId<'ast, UntypedAst>` and `FundefId<'ast, TypedAst>` are distinct types, so an id from
/// one phase's arena can't accidentally be used to index another phase's.
pub type FundefId<'ast, Ast> = Id<Fundef<'ast, Ast>>;

