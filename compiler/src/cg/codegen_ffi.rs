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
}

impl<'ast> Visit<'ast> for CompileFfi {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        self.output.clear();

        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        let mut res = String::new();

        let ret_type = to_rusttype(&fundef.ret_type);

        let args = fundef.args.iter()
            .map(|arg| format!("{}: {}", arg.name, to_rusttype(&arg.ty)))
            .collect::<Vec<String>>()
            .join(", ");

        res.push_str("unsafe extern \"C\" {\n");
        res.push_str(&format!("    fn IMP_{}({}) -> {};\n", fundef.name, args, ret_type));
        res.push_str("}\n\n");

        // Here we have the opportunity to add checks, dispatch to different implementations, etc.
        res.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args, ret_type));
        res.push_str(&format!("    unsafe {{ IMP_{}({}) }}\n",
                                fundef.name,
                                fundef.args.iter().map(|arg| arg.name.to_owned())
                            .collect::<Vec<_>>().join(", ")));
        res.push_str("}\n");

        self.output.push_str(&res);
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
