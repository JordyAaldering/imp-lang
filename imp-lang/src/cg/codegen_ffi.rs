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

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {
        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        self.push("unsafe extern \"C\" {\n");
        self.push(&format!("    fn IMP_{}(", fundef.name));
        self.push(&join_args(&fundef.args, rust_ffi_type));
        self.push(&format!(") -> {};\n", rust_ffi_type(&fundef.ret_type)));
        self.push("}\n");

        // Here we have the opportunity to add checks, dispatch to different implementations, etc.
        self.push(&format!("fn {}(", fundef.name));
        self.push(&join_args(&fundef.args, rust_api_type));
        self.push(&format!(") -> {} {{\n", rust_api_type(&fundef.ret_type)));

        let mut call_args = Vec::with_capacity(fundef.args.len());
        for arg in &fundef.args {
            if is_vector(&arg.ty) {
                self.push(&format!("    let mut __{}_ffi = {};\n", arg.name, arg.name));
                self.push(&format!("    let __{}_raw = __{}_ffi.as_raw();\n", arg.name, arg.name));
                call_args.push(format!("__{}_raw", arg.name));
            } else {
                call_args.push(arg.name.clone());
            }
        }

        if is_vector(&fundef.ret_type) {
            self.push(&format!(
                "    let __raw = unsafe {{ IMP_{}({}) }};\n",
                fundef.name,
                call_args.join(", ")
            ));
            self.push("    unsafe { imp_core::ImpArrayu32::from_raw(__raw) }\n");
        } else {
            self.push(&format!(
                "    unsafe {{ IMP_{}({}) }}\n",
                fundef.name,
                call_args.join(", ")
            ));
        }

        self.push("}\n");
    }
}

fn is_vector(ty: &Type) -> bool {
    matches!(ty.shp, Shape::Vector(_))
}

fn join_args(args: &Vec<&Farg>, map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.name, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_api_type(ty: &Type) -> String {
    let base = match ty.ty {
        BaseType::U32 => "u32",
        BaseType::Bool => "bool",
    };

    match ty.shp {
        Shape::Scalar => base.to_owned(),
        Shape::Vector(_) => format!("imp_core::ImpArray{}", base),
    }
}

fn rust_ffi_type(ty: &Type) -> String {
    let base = match ty.ty {
        BaseType::U32 => "u32",
        BaseType::Bool => "bool",
    };

    match ty.shp {
        Shape::Scalar => base.to_owned(),
        Shape::Vector(_) => format!("imp_core::ImpArray{}Raw", base),
    }
}
