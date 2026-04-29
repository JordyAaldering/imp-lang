use crate::ast::*;

pub fn emit_ffi(ast: &mut Program<'static, TypedAst>) -> String {
    let mut cg = CompileFfi::new();
    cg.trav_program(ast);
    cg.finish()
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

impl<'ast> Traverse<'ast> for CompileFfi {
    type Ast = TypedAst;

    type ExprOut = ();

    const EXPR_DEFAULT: Self::ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) {
        self.push("#[allow(unused_imports)]\n");
        self.push("use imp_core::*;\n");
        self.push("\n");

        self.push("unsafe extern \"C\" {\n");
        for (_name, overloads) in &program.overloads {
            for (_sig, fundefs) in overloads {
                for fundef in fundefs {
                    self.push(&format!("    fn IMP_{}(", fundef.name));
                    self.push(&join_args(&fundef.args, rust_ffi_type));
                    self.push(&format!(") -> {};\n", rust_ffi_type(&fundef.ret_type)));
                }
            }
        }
        self.push("}\n");

        for (name, overloads) in &program.overloads {
            for (sig, fundefs) in overloads {
                self.push("\n");
                if overloads.len() > 1 || fundefs.len() > 1 {
                    self.emit_family_wrapper(&name, sig, fundefs);
                } else {
                    let fundef = fundefs[0];
                    self.emit_direct_wrapper(&name, fundef);
                }
            }
        }
    }
}

impl CompileFfi {
    fn emit_direct_wrapper(&mut self, base_name: &str, fundef: &Fundef<'_, TypedAst>) {
        self.push(&format!("fn {}(", base_name));
        self.push(&join_args(&fundef.args, rust_api_arg_type));
        self.push(&format!(") -> {} {{\n", rust_api_ret_type(&fundef.ret_type)));

        let shape_checks = generate_shape_checks(&fundef.args);
        if !shape_checks.is_empty() {
            self.push(&shape_checks);
        }

        let call_args = emit_marshaled_call_args(&mut self.output, &fundef.args);
        let ret = &emit_return_conversion(&fundef.name, &fundef.ret_type, &call_args);
        for line in ret.lines() {
            self.push("    ");
            self.push(line);
            self.push("\n");
        }
        self.push("}\n");
    }

    fn emit_family_wrapper(&mut self, base_name: &str, sig: &BaseSignature, fundefs: &Vec<&Fundef<'_, TypedAst>>) {
        let sig_str = sig.base_types.iter().map(rust_base_type).collect::<Vec<_>>();
        let fargs = sig.base_types.iter()
            .enumerate()
            .map(|(i, base)| format!("arg{}: ImpArrayOrScalar<{}>", i, rust_base_type(base)))
            .collect::<Vec<_>>()
            .join(", ");

        self.push(&format!("fn {}_{}(", base_name, sig_str.join("_")));
        self.push(&fargs);
        let first = fundefs[0];
        self.push(&format!(") -> {} {{\n", rust_api_ret_type(&first.ret_type)));

        let match_args = &(0..sig.base_types.len())
            .map(|i| format!("arg{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.push("    match (");
        self.push(&match_args);
        self.push(") {\n");

        for fundef in fundefs {
            let pattern = fundef.args.iter()
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

            let branch_args = fundef.args.iter().enumerate().map(|(i, _)| format!("arg{i}")).collect::<Vec<_>>();
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

fn join_args(args: &[Farg], map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.id, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_api_type(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        format!("ImpDyn<{}>", rust_base_type(&ty.ty))
    } else if ty.is_array() {
        format!("ImpArray<{}>", rust_base_type(&ty.ty))
    } else {
        rust_base_type(&ty.ty)
    }
}

fn rust_api_arg_type(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        format!("ImpArrayOrScalar<{}>", rust_base_type(&ty.ty))
    } else {
        rust_api_type(ty)
    }
}

fn rust_api_ret_type(ty: &Type) -> String {
    format!("ImpArrayOrScalar<{}>", rust_base_type(&ty.ty))
}

fn rust_ffi_type(ty: &Type) -> String {
    if matches!(ty.shape, TypePattern::Any) {
        format!("ImpDyn<{}>", rust_base_type(&ty.ty))
    } else if ty.is_array() {
        "ImpArrayRaw".to_owned()
    } else {
        rust_base_type(&ty.ty).to_owned()
    }
}

fn rust_base_type(ty: &BaseType) -> String {
    use BaseType::*;
    match ty {
        Bool => "bool".to_owned(),
        Usize => "usize".to_owned(),
        U32 => "u32".to_owned(),
        U64 => "u64".to_owned(),
        I32 => "i32".to_owned(),
        I64 => "i64".to_owned(),
        F32 => "f32".to_owned(),
        F64 => "f64".to_owned(),
        Udf(udf) => udf.to_owned(),
    }
}

fn emit_marshaled_call_args(out: &mut String, args: &[Farg]) -> Vec<String> {
    let mut call_args = Vec::with_capacity(args.len());
    for arg in args {
        if is_static_array(&arg.ty) {
            out.push_str(&format!("    let {}_raw = {}.into_raw();\n", arg.id, arg.id));
            call_args.push(format!("{}_raw", arg.id));
        } else if matches!(arg.ty.shape, TypePattern::Any) {
            out.push_str(&format!("    let mut {}_dyn = {};\n", arg.id, arg.id));
            out.push_str(&format!("    let {}_ffi = match &mut {}_dyn {{\n", arg.id, arg.id));
            out.push_str("        ImpArrayOrScalar::Scalar(v) => ImpDyn::from_scalar(*v),\n");
            out.push_str("        ImpArrayOrScalar::Array(a) => ImpDyn::from_array_raw(a.into_raw()),\n");
            out.push_str("    };\n");
            call_args.push(format!("{}_ffi", arg.id));
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
            out.push_str(&format!("{pad}let {}_raw = {}.into_raw();\n", branch_name, branch_name));
            call_args.push(format!("{}_raw", branch_name));
        } else if matches!(arg.ty.shape, TypePattern::Any) {
            out.push_str(&format!("{pad}let mut {}_dyn = {};\n", branch_name, branch_name));
            out.push_str(&format!("{pad}let {}_ffi = match &mut {}_dyn {{\n", branch_name, branch_name));
            out.push_str(&format!("{pad}    ImpArrayOrScalar::Scalar(v) => ImpDyn::from_scalar(*v),\n"));
            out.push_str(&format!("{pad}    ImpArrayOrScalar::Array(a) => ImpDyn::from_array_raw(a.into_raw()),\n"));
            out.push_str(&format!("{pad}}};\n"));
            call_args.push(format!("{}_ffi", branch_name));
        } else {
            call_args.push(branch_name.clone());
        }
    }
    call_args
}

fn emit_return_conversion(symbol_name: &str, ret_type: &Type, call_args: &[String]) -> String {
    if matches!(ret_type.shape, TypePattern::Any) {
        format!("let res0_dyn = unsafe {{ IMP_{}({}) }};\nunsafe {{ res0_dyn.into_array_or_scalar() }}",
            symbol_name, call_args.join(", ") )
    } else if is_static_array(ret_type) {
        format!("let res0_raw = unsafe {{ IMP_{}({}) }};\nImpArrayOrScalar::Array(unsafe {{ ImpArray::<{}>::from_raw(res0_raw) }})",
            symbol_name, call_args.join(", "), rust_base_type(&ret_type.ty)
        )
    } else {
        format!("ImpArrayOrScalar::Scalar(unsafe {{ IMP_{}({}) }})",
            symbol_name, call_args.join(", "))
    }
}

fn family_match_pattern(arg_index: usize, ty: &Type) -> String {
    match ty.shape {
        TypePattern::Scalar => format!("ImpArrayOrScalar::Scalar(arg{arg_index})"),
        _ => format!("ImpArrayOrScalar::Array(arg{arg_index})"),
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
                    let binding = format!("_imp_extent_{}", extent);
                    if bound_dims.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!("    assert_eq!({}.shp[{}], {}, \"extent {} mismatch\");\n",
                            arg.id, idx, binding, extent));
                    } else {
                        out.push_str(&format!("    let {} = {}.shp[{}];\n", binding, arg.id, idx));
                        bound_dims.push(binding);
                    }
                }
                AxisPattern::Dim(DimPattern::Any) => {}
                AxisPattern::Rank(capture) => {
                    let binding = format!("_imp_rank_{}", capture.dim_name);
                    if bound_ranks.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!("    assert_eq!({}.shp.len(), {}, \"rank {} mismatch\");\n",
                            arg.id, binding, capture.dim_name));
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
