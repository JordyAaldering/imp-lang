use crate::{Phase, ast::*};

pub fn validate_overloads<'ast>(program: &mut Program<'ast, UntypedAst>) -> Result<(), String> {
    let mut trav = ValidateOverloads::default();
    trav.trav_program(program);
    if trav.errors.is_empty() {
        Ok(())
    } else {
        Err(trav.errors.join("\n"))
    }
}

#[derive(Default)]
struct ValidateOverloads {
    errors: Vec<String>,
}

impl<'ast> Traverse<'ast> for ValidateOverloads {
    const PHASE: Phase = Phase::VO;

    type Ast = UntypedAst;

    type DeclOut = ();

    type ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, Self::Ast>) {
        for (name, family) in &program.overloads.families {
            for (sig, ids) in family {
                let (id, rest) = ids.split_first().unwrap();
                let expected_ret_ty = &program.fundefs[*id].ret_type.basetype;

                for id in rest {
                    let actual_ret_ty = &program.fundefs[*id].ret_type.basetype;
                    if actual_ret_ty != expected_ret_ty {
                        self.errors.push(format!(
                            "Inconsistent return base type for overload family '{}', argument bases {:?}: expected {}, found {}",
                            name,
                            sig,
                            expected_ret_ty,
                            actual_ret_ty,
                        ));
                    }
                }
            }
        }
    }
}
