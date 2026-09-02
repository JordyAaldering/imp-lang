use std::collections::HashMap;

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
    pub overloads: HashMap<String, HashMap<BaseSignature, Vec<FundefId<'ast, Ast>>>>,
    /// Owns every `Fundef` in the program; cross-references use `FundefId` rather
    /// than raw pointers, so this arena can be freely mutated (`iter_mut`) without
    /// invalidating anything that references a fundef by id.
    pub fundefs: id_arena::Arena<Fundef<'ast, Ast>>,
}

impl<'ast, Ast: AstConfig> Program<'ast, Ast> {
    pub fn fundef(&self, id: FundefId<'ast, Ast>) -> &Fundef<'ast, Ast> {
        &self.fundefs[id]
    }

    pub fn fundef_names(&self) -> Vec<String> {
        self.fundefs.iter().map(|(_, f)| f.name.clone()).collect()
    }
}
