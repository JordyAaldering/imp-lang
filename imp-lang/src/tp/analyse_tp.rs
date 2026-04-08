use std::collections::HashSet;

use crate::{ast::*, Rewrite};

/// Does not yet do anything.
pub fn analyse_tp(mut program: Program<'static, ParsedAst>) -> Program<'static, ParsedAst> {
    AnalyseTp::new().rewrite_program(&mut program);
    program
}

struct AnalyseTp {
    /// Symbols that have been defined so far in the current fundef,
    /// accumulated left-to-right across arguments and their type patterns.
    defined: HashSet<String>,
}

impl AnalyseTp {
    fn new() -> Self {
        Self { defined: HashSet::new() }
    }
}

impl Rewrite<'static> for AnalyseTp {
    type Ast = ParsedAst;

    fn rewrite_fundef(&mut self, fundef: &mut Fundef<'static, ParsedAst>) {
        self.defined.clear();

        for arg in &mut fundef.args {
            *arg = self.rewrite_farg(*arg);
        }

        fundef.ret_type = self.rewrite_type(fundef.ret_type.clone());
    }

    fn rewrite_farg(&mut self, arg: &'static Farg) -> &'static Farg {
        arg
    }

    fn rewrite_type(&mut self, ty: Type) -> Type {
        ty
    }
}
