/// A lightweight, `Copy` handle to a `Fundef` stored in `Program.fundefs`.
///
/// Using an index instead of a raw `&'ast Fundef` pointer means cross-function
/// references never alias the `Vec<Fundef>` they point into, so the owning
/// `Program` can freely hand out `&mut Fundef` (e.g. via `iter_mut`) without any
/// unsafe lifetime games.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FundefId(pub usize);
