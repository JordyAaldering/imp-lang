use std::collections::HashMap;
use typed_arena::Arena;

use super::*;

pub struct Program<'ast, Ast: AstConfig> {
    /// Contains all fundefs in the program, grouped by overload.
    ///
    /// A mapping from potentially overloaded function name,
    /// to a mapping from base signature (argument base types without shapes),
    /// to a list of fundefs with that base signature (differing in argument shapes).
    ///
    /// Example:
    /// ```json
    /// {
    ///   "id": {
    ///     (i32) => [ (i32) -> i32 ],
    ///     (f64) => [ (f64) -> f64 ]
    ///   },
    ///   "sel": {
    ///     (usize, i32) => [ (usize[n], i32[n:shp]) -> i32,
    ///                       (usize[n], i32[n:shp,i>0:ishp]) -> i32[i>0:ishp] ],
    ///     (usize, f64) => [ (usize[n], f64[n:shp]) -> f64,
    ///                       (usize[n], f64[n:shp,i>0:ishp]) -> f64[i>0:ishp] ]
    ///   }
    /// }
    /// ```
    pub overloads: HashMap<String, HashMap<BaseSignature, Vec<&'ast Fundef<'ast, Ast>>>>,
    pub fundefs: Arena<Fundef<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> Clone for Program<'ast, Ast> {
    fn clone(&self) -> Self {
        Self {
            overloads: self.overloads.clone(),
            fundefs: Arena::new(),
        }
    }
}
