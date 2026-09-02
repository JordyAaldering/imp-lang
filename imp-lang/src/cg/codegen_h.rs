use crate::ast::*;

pub fn emit_h<'ast>(ast: &mut Program<'ast, TypedAst>) -> String {
    let mut cg = CompileH::new();
    cg.trav_program(ast);
    cg.finish()
}

pub struct CompileH {
    output: String,
}

impl CompileH {
    pub fn new() -> Self {
        Self { output: String::new() }
    }

    pub fn finish(self) -> String {
        self.output
    }
}

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

impl<'ast> Traverse<'ast> for CompileH {
    type Ast = TypedAst;

    type ExprOut = ();

    fn expr_default(&self) -> Self::ExprOut { () }

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) {
        self.output.push_str(HEADER);

        for fundef in program.fundefs.iter_mut() {
            self.trav_fundef(fundef);
        }
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, TypedAst>) {
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", dyn_ctype(&arg.ty), arg.id))
            .collect();
        self.output.push_str(&format!("{} IMP_{}({});\n",
            dyn_ctype(&fundef.ret_type), fundef.name, args.join(", ")
        ));
    }
}

fn base_ctype(ty: &Type) -> String {
    use BaseType::*;
    match &ty.ty {
        Bool => "bool".to_owned(),
        Usize => "size_t".to_owned(),
        U32 => "uint32_t".to_owned(),
        U64 => "uint64_t".to_owned(),
        I32 => "int32_t".to_owned(),
        I64 => "int64_t".to_owned(),
        F32 => "float".to_owned(),
        F64 => "double".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn dyn_ctype(ty: &Type) -> String {
    if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty)
    }
}
