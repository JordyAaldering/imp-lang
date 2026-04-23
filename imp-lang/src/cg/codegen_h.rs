use crate::{ast::*, Traverse};

pub fn emit_h(ast: &mut Program<'static, TypedAst>) -> String {
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

typedef union {
    bool scalar;
    ImpArrayRaw array;
} ImpDynDataBool;
typedef struct {
    bool is_array;
    ImpDynDataBool data;
} ImpDynBool;

typedef union {
    int32_t scalar;
    ImpArrayRaw array;
} ImpDynDataI32;
typedef struct {
    bool is_array;
    ImpDynDataI32 data;
} ImpDynI32;

typedef union {
    int64_t scalar;
    ImpArrayRaw array;
} ImpDynDataI64;
typedef struct {
    bool is_array;
    ImpDynDataI64 data;
} ImpDynI64;

typedef union {
    uint32_t scalar;
    ImpArrayRaw array;
} ImpDynDataU32;
typedef struct {
    bool is_array;
    ImpDynDataU32 data;
} ImpDynU32;

typedef union {
    uint64_t scalar;
    ImpArrayRaw array;
} ImpDynDataU64;
typedef struct {
    bool is_array;
    ImpDynDataU64 data;
} ImpDynU64;

typedef union {
    size_t scalar;
    ImpArrayRaw array;
} ImpDynDataUsize;
typedef struct {
    bool is_array;
    ImpDynDataUsize data;
} ImpDynUsize;

typedef union {
    float scalar;
    ImpArrayRaw array;
} ImpDynDataF32;
typedef struct {
    bool is_array;
    ImpDynDataF32 data;
} ImpDynF32;

typedef union {
    double scalar;
    ImpArrayRaw array;
} ImpDynDataF64;
typedef struct {
    bool is_array;
    ImpDynDataF64 data;
} ImpDynF64;
"#;

impl<'ast> Traverse<'ast> for CompileH {
    type Ast = TypedAst;

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
        I32 => "int32_t".to_owned(),
        I64 => "int64_t".to_owned(),
        U32 => "uint32_t".to_owned(),
        U64 => "uint64_t".to_owned(),
        Usize => "size_t".to_owned(),
        F32 => "float".to_owned(),
        F64 => "double".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn dyn_ctype(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        use BaseType::*;
        match &ty.ty {
            Bool => "ImpDynBool".to_owned(),
            I32 => "ImpDynI32".to_owned(),
            I64 => "ImpDynI64".to_owned(),
            U32 => "ImpDynU32".to_owned(),
            U64 => "ImpDynU64".to_owned(),
            Usize => "ImpDynUsize".to_owned(),
            F32 => "ImpDynF32".to_owned(),
            F64 => "ImpDynF64".to_owned(),
            Udf(udf) => format!("ImpDyn{}", udf),
        }
    } else if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty)
    }
}
