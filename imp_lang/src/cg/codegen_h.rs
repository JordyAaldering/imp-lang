use crate::ast::*;

const HEADER: &str =
r#"#pragma once
#include <stdlib.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct {
    size_t len;
    size_t dim;
    size_t *shp;
    void *data;
} ImpArrayRaw;
"#;

pub fn emit_h(ast: &mut Program<'_, TypedAst>) -> String {
    let mut cg = CompileHeader::default();
    cg.trav_program(ast);
    cg.output
}

#[derive(Default)]
struct CompileHeader {
    output: String,
}

impl<'ast> Traverse<'ast> for CompileHeader {
    type Ast = TypedAst;

    type ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) {
        self.output.push_str(HEADER);

        for (_, fundef) in program.fundefs.iter_mut() {
            self.trav_fundef(fundef);
        }
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, TypedAst>) {
        let args: Vec<String> = fundef.args
            .iter()
            .map(|arg| format!("{} {}", dyn_ctype(&arg.ty), arg.id))
            .collect();

        self.output.push_str(&format!(
            "{} IMP_{}({});\n",
            dyn_ctype(&fundef.ret_type),
            fundef.name,
            args.join(", "),
        ));
    }
}

fn dyn_ctype(ty: &Type) -> String {
    if ty.is_array() {
        "ImpArrayRaw".to_string()
    } else {
        ty.basetype.ctype()
    }
}
