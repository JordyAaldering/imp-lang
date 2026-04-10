use std::path::PathBuf;

use crate::{ast::*, Visit};

pub fn emit_h(ast: &mut Program<'static, TypedAst>, outfile: Option<PathBuf>) {
    let mut cg = CompileH::new();
    cg.visit_program(ast);

    if let Some(outfile) = outfile {
        std::fs::write(outfile, cg.finish()).unwrap();
    }
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

impl<'ast> Visit<'ast> for CompileH {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.output.push_str("#pragma once\n");
        self.output.push_str("#include <stdlib.h>\n");
        self.output.push_str("#include <stdbool.h>\n");
        self.output.push_str("#include <stdint.h>\n");
        self.output.push('\n');
        self.output.push_str("typedef struct {\n");
        self.output.push_str("    size_t len;\n");
        self.output.push_str("    size_t dim;\n");
        self.output.push_str("    size_t *shp;\n");
        self.output.push_str("    void *data;\n");
        self.output.push_str("} ImpArrayRaw;\n");
        self.output.push('\n');
        self.output.push_str("typedef union {\n");
        self.output.push_str("    uint32_t scalar;\n");
        self.output.push_str("    ImpArrayRaw array;\n");
        self.output.push_str("} ImpDynDataU32;\n");
        self.output.push_str("typedef struct {\n");
        self.output.push_str("    bool is_array;\n");
        self.output.push_str("    ImpDynDataU32 data;\n");
        self.output.push_str("} ImpDynU32;\n");
        self.output.push('\n');
        self.output.push_str("typedef union {\n");
        self.output.push_str("    size_t scalar;\n");
        self.output.push_str("    ImpArrayRaw array;\n");
        self.output.push_str("} ImpDynDataUsize;\n");
        self.output.push_str("typedef struct {\n");
        self.output.push_str("    bool is_array;\n");
        self.output.push_str("    ImpDynDataUsize data;\n");
        self.output.push_str("} ImpDynUsize;\n");
        self.output.push('\n');
        self.output.push_str("typedef union {\n");
        self.output.push_str("    bool scalar;\n");
        self.output.push_str("    ImpArrayRaw array;\n");
        self.output.push_str("} ImpDynDataBool;\n");
        self.output.push_str("typedef struct {\n");
        self.output.push_str("    bool is_array;\n");
        self.output.push_str("    ImpDynDataBool data;\n");
        self.output.push_str("} ImpDynBool;\n");

        let mut func_names: Vec<&str> = program.functions.keys().map(String::as_str).collect();
        func_names.sort();
        for name in func_names {
            let fundef = &program.functions[name];
            self.output.push('\n');
            self.visit_fundef(fundef);
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", full_ctype(&arg.ty), arg.id))
            .collect();
        self.output.push_str(&format!(
            "{} IMP_{}({});\n",
            full_ctype(&fundef.ret_type), fundef.name, args.join(", ")
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

fn full_ctype(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        use BaseType::*;
        return match &ty.ty {
            Bool => "ImpDynBool".to_owned(),
            I32 => "ImpDynI32".to_owned(),
            I64 => "ImpDynI64".to_owned(),
            U32 => "ImpDynU32".to_owned(),
            U64 => "ImpDynU64".to_owned(),
            Usize => "ImpDynUsize".to_owned(),
            F32 => "ImpDynF32".to_owned(),
            F64 => "ImpDynF64".to_owned(),
            Udf(udf) => format!("ImpDyn{}", udf),
        };
    }

    if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty).to_owned()
    }
}
