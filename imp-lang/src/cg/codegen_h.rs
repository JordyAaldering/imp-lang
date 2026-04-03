use crate::{ast::*, Visit};

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

        let mut func_names: Vec<&str> = program.fundefs.keys().map(String::as_str).collect();
        func_names.sort();
        for name in func_names {
            let wrapper = &program.fundefs[name];
            for fundef in &wrapper.overloads {
                self.output.push('\n');
                self.visit_fundef(fundef);
            }
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        let args: Vec<String> = fundef.args.iter()
            .map(|arg| format!("{} {}", full_ctype(&arg.ty), arg.name))
            .collect();
        self.output.push_str(&format!(
            "{} IMP_{}({});\n",
            full_ctype(&fundef.ret_type), fundef.name, args.join(", ")
        ));
    }
}

fn base_ctype(ty: &Type) -> &'static str {
    match ty.ty {
        BaseType::U32 => "uint32_t",
        BaseType::Usize => "size_t",
        BaseType::Bool => "bool",
    }
}

fn full_ctype(ty: &Type) -> String {
    if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        base_ctype(ty).to_owned()
    }
}
