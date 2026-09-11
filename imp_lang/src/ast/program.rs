use std::collections::HashMap;

use super::*;

pub struct Program<'ast, Ast: Invariant> {
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
    pub overloads: OverloadFamilies<'ast, Ast>,
    /// Owns every [`Fundef`] in the program.
    ///
    /// References use [`FundefId`] rather than raw pointers, so this arena can be freely mutated.
    pub fundefs: id_arena::Arena<Fundef<'ast, Ast>>,
}

pub struct OverloadFamilies<'ast, Ast>
where
    Ast: Invariant,
{
    /// Contains all fundefs in the program, grouped by overload.
    ///
    /// Maps each function name to a HashMap that contains all overloads of that function,
    /// grouped by the base types of their arguments (ignoring shapes). Functions with the
    /// same argument base types are required to have the same return base type.
    /// This second HashMap contains for each unique base signature combination a list
    /// of all fundefs that have that base signature, which may differ in argument shapes.
    pub families: HashMap<String, HashMap<BaseSignature, Vec<FundefId<'ast, Ast>>>>
}

impl<'ast, Ast: Invariant> Program<'ast, Ast> {
    pub fn fundef(&self, id: FundefId<'ast, Ast>) -> &Fundef<'ast, Ast> {
        &self.fundefs[id]
    }

    pub fn fundef_names(&self) -> Vec<String> {
        self.fundefs.iter().map(|(_, f)| f.name.clone()).collect()
    }
}

impl<'ast, Ast: Invariant> OverloadFamilies<'ast, Ast> {
    pub fn flatten(&self) -> impl Iterator<Item = (&String, &BaseSignature, &Vec<FundefId<'ast, Ast>>)> {
        self.families
            .iter()
            .flat_map(|(name, family)| {
                debug_assert!(!family.is_empty());
                family.iter()
                    .map(move |(signature, overloads)| {
                        debug_assert!(!overloads.is_empty());
                        (name, signature, overloads)
                    })
            })
    }
}
