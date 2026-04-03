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
        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub union ImpDynDataU32 {\n");
        self.push("    pub scalar: u32,\n");
        self.push("    pub array: imp_core::ImpArrayRaw,\n");
        self.push("}\n");
        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub struct ImpDynU32 {\n");
        self.push("    pub is_array: bool,\n");
        self.push("    pub data: ImpDynDataU32,\n");
        self.push("}\n");

        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub union ImpDynDataUsize {\n");
        self.push("    pub scalar: usize,\n");
        self.push("    pub array: imp_core::ImpArrayRaw,\n");
        self.push("}\n");
        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub struct ImpDynUsize {\n");
        self.push("    pub is_array: bool,\n");
        self.push("    pub data: ImpDynDataUsize,\n");
        self.push("}\n");

        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub union ImpDynDataBool {\n");
        self.push("    pub scalar: bool,\n");
        self.push("    pub array: imp_core::ImpArrayRaw,\n");
        self.push("}\n");
        self.push("#[repr(C)]\n");
        self.push("#[derive(Clone, Copy)]\n");
        self.push("pub struct ImpDynBool {\n");
        self.push("    pub is_array: bool,\n");
        self.push("    pub data: ImpDynDataBool,\n");
        self.push("}\n");

        for wrapper in program.fundefs.values() {
            for fundef in &wrapper.overloads {
                self.push("#[allow(dead_code)]\n");
                self.push("unsafe extern \"C\" {\n");
                self.push(&format!("    fn IMP_{}(", fundef.name));
                self.push(&join_args(&fundef.args, rust_ffi_type));
                self.push(&format!(") -> {};\n", rust_ffi_type(&fundef.ret_type)));
                self.push("}\n");
            }

            // For now we expose one wrapper API symbol in Rust.
            // Overload-aware Rust dispatch can be added later.
            if let Some(primary) = wrapper.overloads.first() {
                self.push("#[allow(dead_code)]\n");
                self.push(&format!("fn {}(", wrapper.name));
                self.push(&join_args(&primary.args, rust_api_type));
                self.push(&format!(") -> {} {{\n", rust_api_type(&primary.ret_type)));

                let mut call_args = Vec::with_capacity(primary.args.len());
                for arg in &primary.args {
                    if is_vector(&arg.ty) {
                        self.push(&format!("    let mut __{}_ffi = {};\n", arg.name, arg.name));
                        self.push(&format!("    let __{}_raw = __{}_ffi.as_raw();\n", arg.name, arg.name));
                        call_args.push(format!("__{}_raw", arg.name));
                    } else {
                        call_args.push(arg.name.clone());
                    }
                }

                if is_vector(&primary.ret_type) {
                    self.push(&format!(
                        "    let __raw = unsafe {{ IMP_{}({}) }};\n",
                        primary.name,
                        call_args.join(", ")
                    ));
                    self.push(&format!(
                        "    unsafe {{ imp_core::ImpArray::<{}>::from_raw(__raw) }}\n",
                        rust_base_type(&primary.ret_type)
                    ));
                } else {
                    self.push(&format!(
                        "    unsafe {{ IMP_{}({}) }}\n",
                        primary.name,
                        call_args.join(", ")
                    ));
                }

                self.push("}\n");
            }
        }
    }
}

fn is_vector(ty: &Type) -> bool {
    ty.is_vector()
}

fn join_args(args: &Vec<&Farg>, map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.name, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_api_type(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        return match ty.ty {
            BaseType::U32 => "ImpDynU32".to_owned(),
            BaseType::Usize => "ImpDynUsize".to_owned(),
            BaseType::Bool => "ImpDynBool".to_owned(),
        };
    }

    let base = rust_base_type(ty);

    if ty.is_vector() {
        format!("imp_core::ImpArray<{}>", base)
    } else {
        base.to_owned()
    }
}

fn rust_ffi_type(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        return match ty.ty {
            BaseType::U32 => "ImpDynU32".to_owned(),
            BaseType::Usize => "ImpDynUsize".to_owned(),
            BaseType::Bool => "ImpDynBool".to_owned(),
        };
    }

    if ty.is_vector() {
        "imp_core::ImpArrayRaw".to_owned()
    } else {
        rust_base_type(ty).to_owned()
    }
}

fn rust_base_type(ty: &Type) -> &'static str {
    match ty.ty {
        BaseType::U32 => "u32",
        BaseType::Usize => "usize",
        BaseType::Bool => "bool",
    }
}
