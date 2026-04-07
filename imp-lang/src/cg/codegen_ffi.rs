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
        self.push("#[allow(unused_imports)]\n");
        self.push("use imp_core::*;\n");

        for (base_name, fundef) in &program.functions {
            self.push("#[allow(dead_code)]\n");
            self.push("unsafe extern \"C\" {\n");
            self.push(&format!("    fn IMP_{}(", fundef.name));
            self.push(&join_args(&fundef.args, rust_ffi_type));
            self.push(&format!(") -> {};\n", rust_ffi_type(&fundef.ret_type)));
            self.push("}\n");

            self.push("#[allow(dead_code)]\n");
            self.push(&format!("fn {}(", base_name));
            self.push(&join_args(&fundef.args, rust_api_arg_type));
            self.push(&format!(") -> {} {{\n", rust_api_ret_type(&fundef.ret_type)));

            let mut call_args = Vec::with_capacity(fundef.args.len());
            for arg in &fundef.args {
                if is_static_array(&arg.ty) {
                    self.push(&format!("    let mut __{}_ffi = {};\n", arg.name, arg.name));
                    self.push(&format!("    let __{}_raw = __{}_ffi.as_raw();\n", arg.name, arg.name));
                    call_args.push(format!("__{}_raw", arg.name));
                } else if matches!(arg.ty.shape, ShapePattern::Any) {
                    self.push(&format!("    let mut __{}_dyn = {};\n", arg.name, arg.name));
                    self.push(&format!("    let __{}_ffi = match &mut __{}_dyn {{\n", arg.name, arg.name));
                    self.push("        imp_core::ImpArrayOrScalar::Scalar(v) => imp_core::ImpDyn::from_scalar(*v),\n");
                    self.push("        imp_core::ImpArrayOrScalar::Array(a) => imp_core::ImpDyn::from_array_raw(a.as_raw()),\n");
                    self.push("    };\n");
                    call_args.push(format!("__{}_ffi", arg.name));
                } else {
                    call_args.push(arg.name.clone());
                }
            }

            if matches!(fundef.ret_type.shape, ShapePattern::Any) {
                self.push(&format!(
                    "    let __dyn = unsafe {{ IMP_{}({}) }};\n",
                    fundef.name,
                    call_args.join(", ")
                ));
                self.push("    unsafe { __dyn.into_array_or_scalar() }\n");
            } else if is_static_array(&fundef.ret_type) {
                self.push(&format!(
                    "    let __raw = unsafe {{ IMP_{}({}) }};\n",
                    fundef.name,
                    call_args.join(", ")
                ));
                self.push(&format!(
                    "    imp_core::ImpArrayOrScalar::Array(unsafe {{ imp_core::ImpArray::<{}>::from_raw(__raw) }})\n",
                    rust_base_type(&fundef.ret_type)
                ));
            } else {
                self.push(&format!(
                    "    imp_core::ImpArrayOrScalar::Scalar(unsafe {{ IMP_{}({}) }})\n",
                    fundef.name,
                    call_args.join(", ")
                ));
            }

            self.push("}\n");
        }
    }
}

fn is_static_array(ty: &Type) -> bool {
    ty.is_array() && !matches!(ty.shape, ShapePattern::Any)
}

fn join_args(args: &Vec<&Farg>, map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.name, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_api_type(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        return format!("imp_core::ImpDyn<{}>", rust_base_type(ty));
    }

    let base = rust_base_type(ty);

    if ty.is_array() {
        format!("imp_core::ImpArray<{}>", base)
    } else {
        base.to_owned()
    }
}

fn rust_api_arg_type(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        format!("imp_core::ImpArrayOrScalar<{}>", rust_base_type(ty))
    } else {
        rust_api_type(ty)
    }
}

fn rust_api_ret_type(ty: &Type) -> String {
    format!("imp_core::ImpArrayOrScalar<{}>", rust_base_type(ty))
}

fn rust_ffi_type(ty: &Type) -> String {
    if matches!(ty.shape, ShapePattern::Any) {
        return format!("imp_core::ImpDyn<{}>", rust_base_type(ty));
    }

    if ty.is_array() {
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
