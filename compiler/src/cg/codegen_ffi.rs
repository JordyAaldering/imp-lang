use crate::{ast::*, traverse::Visit};

pub struct CompileFfi {
    output: String,
}

impl CompileFfi {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn finish(self) -> String {
        self.output
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

impl<'ast> Visit<'ast> for CompileFfi {
    type Ast = TypedAst;

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        self.push("unsafe extern \"C\" {\n");
        self.push(&format!("    fn IMP_{}(", fundef.name));
        self.visit_fargs(&fundef.args);
        self.push(&format!(") -> {};\n", to_rusttype(&fundef.ret_type)));
        self.push("}\n");

        // Here we have the opportunity to add checks, dispatch to different implementations, etc.
        self.push(&format!("fn {}(", fundef.name));
        self.visit_fargs(&fundef.args);
        self.push(&format!(") -> {} {{\n", to_rusttype(&fundef.ret_type)));

        self.push(&format!("    unsafe {{ IMP_{}({}) }}\n",
                                fundef.name,
                                fundef.args.iter().map(|arg| arg.name.to_owned())
                            .collect::<Vec<_>>().join(", ")));

        self.push("}\n");
    }

    fn visit_farg(&mut self, arg: &'ast Farg<Self::Ast>) {
        self.push(&format!("{}: {}, ", arg.name, to_rusttype(&arg.ty)));
    }
}

fn to_rusttype(ty: &Type) -> String {
    let base = match ty.basetype {
        BaseType::U32 => "u32",
        BaseType::Bool => "bool",
    };

    match ty.shp {
        Shape::Scalar => base.to_owned(),
        Shape::Vector(_) => format!("Vec<{}>", base),
    }
}
