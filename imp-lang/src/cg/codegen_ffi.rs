use std::path::PathBuf;

use crate::{ast::*, traverse::Visit};

pub fn emit_ffi(ast: &mut Program<'static, TypedAst>, outfile: Option<PathBuf>) {
    let mut cg = CompileFfi::new();
    cg.visit_program(ast);

    if let Some(outfile) = outfile {
        std::fs::write(outfile, cg.finish()).unwrap();
    }
}

struct PublicFamily<'prog, 'ast> {
    root_name: String,
    overloads: Vec<(String, &'prog Fundef<'ast, TypedAst>)>,
}

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

        let families = collect_public_families(program);

        for (_base_name, fundef) in program.functions.iter() {
            self.push("\n");
            self.push("unsafe extern \"C\" {\n");
            self.push(&format!("    fn IMP_{}(", fundef.name));
            self.push(&join_args(&fundef.args, rust_ffi_type));
            self.push(&format!(") -> {};\n", rust_ffi_type(&fundef.ret_type)));
            self.push("}\n");
        }

        for family in families {
            if can_emit_family_wrapper(&family) {
                self.emit_family_wrapper(&family);
            } else {
                for (base_name, fundef) in family.overloads {
                    self.emit_direct_wrapper(&base_name, fundef);
                }
            }
        }
    }
}

impl CompileFfi {
    fn emit_direct_wrapper(&mut self, base_name: &str, fundef: &Fundef<'_, TypedAst>) {
        self.push("\n");
        self.push(&format!("fn {}(", rust_wrapper_name(base_name)));
        self.push(&join_args(&fundef.args, rust_api_arg_type));
        self.push(&format!(") -> {} {{\n", rust_api_ret_type(&fundef.ret_type)));

        let shape_checks = generate_shape_checks(&fundef.args);
        if !shape_checks.is_empty() {
            self.push(&shape_checks);
        }

        let call_args = emit_marshaled_call_args(&mut self.output, &fundef.args);
        self.push(&emit_return_conversion(&fundef.name, &fundef.ret_type, &call_args));
        self.push("\n");
        self.push("}\n");
    }

    fn emit_family_wrapper(&mut self, family: &PublicFamily<'_, '_>) {
        let first = family.overloads[0].1;
        let arg_bases: Vec<String> = first.args.iter().map(|a| rust_base_type(&a.ty)).collect();

        self.push("\n");
        self.push(&format!("fn {}(", rust_wrapper_name(&family.root_name)));
        self.push(
            &arg_bases
                .iter()
                .enumerate()
                .map(|(i, base)| format!("arg{i}: imp_core::ImpArrayOrScalar<{base}>"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        self.push(&format!(") -> {} {{\n", rust_api_ret_type(&first.ret_type)));
        self.push("    match (");
        self.push(
            &(0..arg_bases.len())
                .map(|i| format!("arg{i}"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        self.push(") {\n");

        for (_base_name, fundef) in &family.overloads {
            let pattern = fundef
                .args
                .iter()
                .enumerate()
                .map(|(i, arg)| family_match_pattern(i, &arg.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let guard = family_match_guard(&fundef.args);
            self.push("        (");
            self.push(&pattern);
            self.push(")");
            if !guard.is_empty() {
                self.push(" if ");
                self.push(&guard);
            }
            self.push(" => {\n");

            let branch_args = fundef.args.iter().enumerate().map(|(i, _)| format!("arg{i}" )).collect::<Vec<_>>();
            let marshaled = emit_marshaled_branch_args(&mut self.output, &fundef.args, &branch_args, 3);
            let ret = emit_return_conversion(&fundef.name, &fundef.ret_type, &marshaled);
            for line in ret.lines() {
                self.push("            ");
                self.push(line);
                self.push("\n");
            }
            self.push("        }\n");
        }

        self.push("        _ => panic!(\"runtime overload dispatch failed\"),\n");
        self.push("    }\n");
        self.push("}\n");
    }
}

fn is_static_array(ty: &Type) -> bool {
    ty.is_array() && !matches!(ty.shape, TypePattern::Any)
}

fn collect_public_families<'prog, 'ast>(program: &'prog Program<'ast, TypedAst>) -> Vec<PublicFamily<'prog, 'ast>> {
    let mut keys: Vec<_> = program.functions.keys().cloned().collect();
    keys.sort();

    let mut families: Vec<PublicFamily<'prog, 'ast>> = Vec::new();
    for key in keys {
        let fundef = &program.functions[&key];

        let root = key.split("__ovl").next().unwrap_or(&key).to_owned();
        if let Some(existing) = families.iter_mut().find(|family| family.root_name == root) {
            existing.overloads.push((key, fundef));
        } else {
            families.push(PublicFamily {
                root_name: root,
                overloads: vec![(key, fundef)],
            });
        }
    }

    families
}

fn can_emit_family_wrapper(family: &PublicFamily<'_, '_>) -> bool {
    if family.overloads.len() < 2 {
        return false;
    }

    let first = family.overloads[0].1;
    family.overloads.iter().all(|(_, fundef)| {
        fundef.args.len() == first.args.len()
            && fundef.ret_type.ty == first.ret_type.ty
            && fundef.args.iter().zip(first.args.iter()).all(|(a, b)| a.ty.ty == b.ty.ty)
    })
}

fn join_args(args: &[Farg], map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.id, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_api_type(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
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
    if matches!(ty.shape, TypePattern::Any) {
        format!("imp_core::ImpArrayOrScalar<{}>", rust_base_type(ty))
    } else {
        rust_api_type(ty)
    }
}

fn rust_api_ret_type(ty: &Type) -> String {
    format!("imp_core::ImpArrayOrScalar<{}>", rust_base_type(ty))
}

fn rust_ffi_type(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        return format!("imp_core::ImpDyn<{}>", rust_base_type(ty));
    }

    if ty.is_array() {
        "imp_core::ImpArrayRaw".to_owned()
    } else {
        rust_base_type(ty).to_owned()
    }
}

fn rust_base_type(ty: &Type) -> String {
    use BaseType::*;
    match &ty.ty {
        Bool => "bool".to_owned(),
        I32 => "i32".to_owned(),
        I64 => "i64".to_owned(),
        U32 => "u32".to_owned(),
        U64 => "u64".to_owned(),
        Usize => "usize".to_owned(),
        F32 => "f32".to_owned(),
        F64 => "f64".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn rust_wrapper_name(name: &str) -> String {
    name.strip_prefix('@').unwrap_or(name).to_owned()
}

fn emit_marshaled_call_args(out: &mut String, args: &[Farg]) -> Vec<String> {
    let mut call_args = Vec::with_capacity(args.len());
    for arg in args {
        if is_static_array(&arg.ty) {
            out.push_str(&format!("    let mut __{}_ffi = {};\n", arg.id, arg.id));
            out.push_str(&format!("    let __{}_raw = __{}_ffi.as_raw();\n", arg.id, arg.id));
            call_args.push(format!("__{}_raw", arg.id));
        } else if matches!(arg.ty.shape, TypePattern::Any) {
            out.push_str(&format!("    let mut __{}_dyn = {};\n", arg.id, arg.id));
            out.push_str(&format!("    let __{}_ffi = match &mut __{}_dyn {{\n", arg.id, arg.id));
            out.push_str("        imp_core::ImpArrayOrScalar::Scalar(v) => imp_core::ImpDyn::from_scalar(*v),\n");
            out.push_str("        imp_core::ImpArrayOrScalar::Array(a) => imp_core::ImpDyn::from_array_raw(a.as_raw()),\n");
            out.push_str("    };\n");
            call_args.push(format!("__{}_ffi", arg.id));
        } else {
            call_args.push(arg.id.clone());
        }
    }
    call_args
}

fn emit_marshaled_branch_args(out: &mut String, args: &[Farg], branch_names: &[String], indent: usize) -> Vec<String> {
    let pad = "    ".repeat(indent);
    let mut call_args = Vec::with_capacity(args.len());
    for (arg, branch_name) in args.iter().zip(branch_names.iter()) {
        if is_static_array(&arg.ty) {
            out.push_str(&format!("{pad}let mut __{}_ffi = {};\n", branch_name, branch_name));
            out.push_str(&format!("{pad}let __{}_raw = __{}_ffi.as_raw();\n", branch_name, branch_name));
            call_args.push(format!("__{}_raw", branch_name));
        } else if matches!(arg.ty.shape, TypePattern::Any) {
            out.push_str(&format!("{pad}let mut __{}_dyn = {};\n", branch_name, branch_name));
            out.push_str(&format!("{pad}let __{}_ffi = match &mut __{}_dyn {{\n", branch_name, branch_name));
            out.push_str(&format!("{pad}    imp_core::ImpArrayOrScalar::Scalar(v) => imp_core::ImpDyn::from_scalar(*v),\n"));
            out.push_str(&format!("{pad}    imp_core::ImpArrayOrScalar::Array(a) => imp_core::ImpDyn::from_array_raw(a.as_raw()),\n"));
            out.push_str(&format!("{pad}}};\n"));
            call_args.push(format!("__{}_ffi", branch_name));
        } else {
            call_args.push(branch_name.clone());
        }
    }
    call_args
}

fn emit_return_conversion(symbol_name: &str, ret_type: &Type, call_args: &[String]) -> String {
    if matches!(ret_type.shape, TypePattern::Any) {
        format!(
            "    let __dyn = unsafe {{ IMP_{}({}) }};\n    unsafe {{ __dyn.into_array_or_scalar() }}",
            symbol_name,
            call_args.join(", ")
        )
    } else if is_static_array(ret_type) {
        format!(
            "    let __raw = unsafe {{ IMP_{}({}) }};\n    imp_core::ImpArrayOrScalar::Array(unsafe {{ imp_core::ImpArray::<{}>::from_raw(__raw) }})",
            symbol_name,
            call_args.join(", "),
            rust_base_type(ret_type)
        )
    } else {
        format!(
            "    imp_core::ImpArrayOrScalar::Scalar(unsafe {{ IMP_{}({}) }})",
            symbol_name,
            call_args.join(", ")
        )
    }
}

fn family_match_pattern(arg_index: usize, ty: &Type) -> String {
    match ty.shape {
        TypePattern::Scalar => format!("imp_core::ImpArrayOrScalar::Scalar(arg{arg_index})"),
        _ => format!("imp_core::ImpArrayOrScalar::Array(arg{arg_index})"),
    }
}

fn family_match_guard(args: &[Farg]) -> String {
    let mut checks = Vec::new();
    let mut bound_dims: Vec<(String, String)> = Vec::new();
    let mut bound_ranks: Vec<(String, String)> = Vec::new();

    for (arg_index, arg) in args.iter().enumerate() {
        let TypePattern::Axes(axes) = &arg.ty.shape else {
            continue;
        };

        if !axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))) {
            checks.push(format!("arg{arg_index}.shp.len() == {}", axes.len()));
        }

        for (axis_index, axis) in axes.iter().enumerate() {
            match axis {
                AxisPattern::Dim(DimPattern::Known(v)) => {
                    checks.push(format!("arg{arg_index}.shp[{axis_index}] == {v}"));
                }
                AxisPattern::Dim(DimPattern::Var(extent)) => {
                    let expr = format!("arg{arg_index}.shp[{axis_index}]");
                    if let Some((_, bound_expr)) = bound_dims.iter().find(|(name, _)| name == extent) {
                        checks.push(format!("{expr} == {bound_expr}"));
                    } else {
                        bound_dims.push((extent.clone(), expr));
                    }
                }
                AxisPattern::Dim(DimPattern::Any) => {}
                AxisPattern::Rank(capture) => {
                    let expr = format!("arg{arg_index}.shp.len()");
                    if let Some((_, bound_expr)) = bound_ranks.iter().find(|(name, _)| name == &capture.dim_name) {
                        checks.push(format!("{expr} == {bound_expr}"));
                    } else {
                        bound_ranks.push((capture.dim_name.clone(), expr));
                    }
                }
            }
        }
    }

    checks.join(" && ")
}

fn generate_shape_checks(args: &[Farg]) -> String {
    let mut out = String::new();
    let mut bound_dims: Vec<String> = Vec::new();
    let mut bound_ranks: Vec<String> = Vec::new();

    for arg in args {
        let TypePattern::Axes(axes) = &arg.ty.shape else {
            continue;
        };

        if !axes.iter().any(|axis| matches!(axis, AxisPattern::Rank(_))) {
            out.push_str(&format!(
                "    assert_eq!({}.shp.len(), {}, \"{} rank mismatch\");\n",
                arg.id,
                axes.len(),
                arg.id,
            ));
        }

        for (idx, axis) in axes.iter().enumerate() {
            match axis {
                AxisPattern::Dim(DimPattern::Known(v)) => {
                    out.push_str(&format!(
                        "    assert_eq!({}.shp[{}], {}, \"{} extent mismatch at axis {}\");\n",
                        arg.id,
                        idx,
                        v,
                        arg.id,
                        idx,
                    ));
                }
                AxisPattern::Dim(DimPattern::Var(extent)) => {
                    let binding = format!("__imp_extent_{}", sanitize_binding_name(&extent));
                    if bound_dims.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!(
                            "    assert_eq!({}.shp[{}], {}, \"extent {} mismatch\");\n",
                            arg.id,
                            idx,
                            binding,
                            extent,
                        ));
                    } else {
                        out.push_str(&format!("    let {} = {}.shp[{}];\n", binding, arg.id, idx));
                        bound_dims.push(binding);
                    }
                }
                AxisPattern::Dim(DimPattern::Any) => {}
                AxisPattern::Rank(capture) => {
                    let binding = format!("__imp_rank_{}", sanitize_binding_name(&capture.dim_name));
                    if bound_ranks.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!(
                            "    assert_eq!({}.shp.len(), {}, \"rank {} mismatch\");\n",
                            arg.id,
                            binding,
                            capture.dim_name,
                        ));
                    } else {
                        out.push_str(&format!("    let {} = {}.shp.len();\n", binding, arg.id));
                        bound_ranks.push(binding);
                    }
                }
            }
        }
    }

    out
}

fn sanitize_binding_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
