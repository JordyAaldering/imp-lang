use crate::{ast::*, traverse::Visit};

/// With function overloading, we have a few cases.
///
/// 1) The types are the same, but the type patterns differ.
///
/// ```
/// fn iota(usize n) -> usize[n] {
///     return { iv[[0]] | [0] <= iv < [n] };
/// }
///
/// fn iota(usize[d] shp) -> usize[d:shp,d] {
///     return { iv | (0*shp) <= iv < shp };
/// }
/// ```
///
/// For this, the C looks somewhat like:
///
/// ```
/// ImpArrayRaw IMP_iota__usize_0(size_t n) { ... }
///
/// ImpArrayRaw IMP_iota__usize_d(ImpArrayRaw n) { ... }
///
/// ImpArrayRaw IMP_iota__usize(union n) {
///     // Generated wrapper function that checks `n` and dispatches to the correct overload.
/// }
/// ```
///
/// Although from the rust side we could just call the wrapper, we actually want to generate the same logic again
/// The reason for this is because we deliberately do not generate checks in the C world.
/// Once we have entered C-land, assuming that the input is correct, we should be pretty sure that our code is valid
/// if our type patterns are explicit enough.
///
/// We still need the wrappers in the C side, as C functions may themselves call wrappers.
///
/// That does mean that this assumption of correct input must hold.
/// If this is not the case, we don't want to abort.
/// Thus, we regenerate the wrapper in Rust, adding the additional checks to ensure that the shapes are correct.
///
/// 2) The base-types may differ
///
/// ```
/// fn myadd(u32 x, u32 y) -> u32 { ...}
///
/// fn myadd(usize x, usize y) -> usize { ... }
/// ```
///
/// It is not yet entirely clear what happens. Does this return a union type? For now, let's assume so
///
/// ```
/// u32_union IMP_myadd__u32_0(u32_union n) { ... }
///
/// usize_union IMP_myadd__usize_d(usize_union n) { ... }
/// ```
///
/// Now, we cannot have a single Rust function that dispatches to the correct overload, as the argument types differ.
/// That is, unless we put everything in an enum on the rust side.
/// But perhaps generating a trait on the rust side is a better idea.
///
/// 3) even argument counts may differ?
///    It is not yet clear if argument counts will ever actually differ.
///    If we disallow direct overloading, but instead support some kind of "traits",
///    then at least the argument count stays the same.
///
/// ---
///
/// Clearly, there is still lots to figure out.
/// First, some other things, like the union types and choice for argument counts, need to stabalise.
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
