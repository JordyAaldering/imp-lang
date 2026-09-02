use crate::ast::*;

pub fn emit_ffi(ast: &mut Program<'_, TypedAst>) -> String {
    let mut cg = CompileFfi::default();
    cg.trav_program(ast);
    cg.output
}

#[derive(Default)]
struct CompileFfi {
    output: String,
}

impl CompileFfi {
    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

impl<'ast> Traverse<'ast> for CompileFfi {
    type Ast = TypedAst;

    type ExprOut = ();

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) {
        self.push("#[allow(unused_imports)]\n");
        self.push("use imp_core::*;\n");
        self.push("\n");

        self.push("unsafe extern \"C\" {\n");
        for (_name, overloads) in &program.overloads {
            for (_sig, fundef_ids) in overloads {
                for fundef_id in fundef_ids {
                    let fundef = program.fundef(*fundef_id);
                    self.push(&format!("    fn IMP_{}(", fundef.name));
                    self.push(&join_args(&fundef.args, Type::rstype));
                    self.push(&format!(") -> {};\n", fundef.ret_type.rstype()));
                }
            }
        }
        self.push("}\n");

        for (name, overloads) in &program.overloads {
            for (sig, fundef_ids) in overloads {
                self.push("\n");
                let fundefs: Vec<&Fundef<TypedAst>> = fundef_ids.iter().map(|&id| program.fundef(id)).collect();
                if overloads.len() > 1 || fundefs.len() > 1 {
                    self.emit_family_wrapper(&name, sig, &fundefs);
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
        let ret_ty_str = rust_wrapper_type(&fundef.ret_type);
        self.push(&format!("fn {}(", base_name));
        self.push(&join_args(&fundef.args, rust_wrapper_type));
        self.push(&format!(") -> {ret_ty_str} {{\n"));

        let shape_checks = generate_shape_checks(&fundef.args);
        if !shape_checks.is_empty() {
            self.push(&shape_checks);
        }

        let call_args = emit_marshaled_call_args(&mut self.output, &fundef.args);
        let ret = emit_return_expr(&fundef.name, &fundef.ret_type, &call_args);
        for line in ret.lines() {
            self.push("    ");
            self.push(line);
            self.push("\n");
        }
        self.push("}\n");
    }

    fn emit_family_wrapper(
        &mut self,
        base_name: &str,
        sig: &BaseSignature,
        fundefs: &Vec<&Fundef<'_, TypedAst>>,
    ) {
        let sig_str = sig.base_types.iter().map(BaseType::rstype).collect::<Vec<_>>();

        // Per-position: is this arg scalar for ALL variants?
        let n_args = sig.base_types.len();
        let all_scalar_args: Vec<bool> = (0..n_args)
            .map(|i| fundefs.iter().all(|f| f.args[i].ty.is_scalar()))
            .collect();
        let all_scalar_ret = fundefs.iter().all(|f| f.ret_type.is_scalar());

        let fargs = sig
            .base_types
            .iter()
            .enumerate()
            .map(|(i, base)| {
                if all_scalar_args[i] {
                    format!("arg{}: {}", i, base.rstype())
                } else {
                    format!("arg{}: ImpArray<{}>", i, base.rstype())
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let first = fundefs[0];
        let ret_ty_str = if all_scalar_ret {
            first.ret_type.basetype.rstype()
        } else {
            format!("ImpArray<{}>", first.ret_type.basetype.rstype())
        };

        self.push(&format!(
            "fn {}_{}({}) -> {ret_ty_str} {{\n",
            base_name,
            sig_str.join("_"),
            fargs
        ));

        for (idx, fundef) in fundefs.iter().enumerate() {
            let condition = build_variant_condition(&fundef.args, &all_scalar_args);
            if idx == 0 {
                self.push(&format!("    if {condition} {{\n"));
            } else {
                self.push(&format!("    }} else if {condition} {{\n"));
            }

            let branch_arg_names: Vec<String> =
                (0..n_args).map(|i| format!("arg{i}")).collect();
            let marshaled = emit_marshaled_branch_args(
                &mut self.output,
                &fundef.args,
                &branch_arg_names,
                &all_scalar_args,
                2,
            );
            let ret = emit_family_return_expr(
                &fundef.name,
                &fundef.ret_type,
                &marshaled,
                all_scalar_ret,
            );
            for line in ret.lines() {
                self.push("        ");
                self.push(line);
                self.push("\n");
            }
        }

        self.push("    } else {\n");
        self.push(&format!(
            "        panic!(\"runtime overload dispatch failed: {}\");\n",
            base_name
        ));
        self.push("    }\n");
        self.push("}\n");
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn is_static_array(ty: &Type) -> bool {
    ty.is_array()
}

fn join_args(args: &[Farg], map_ty: fn(&Type) -> String) -> String {
    args.iter()
        .map(|arg| format!("{}: {}", arg.id, map_ty(&arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rust type used in wrapper signatures: arrays -> `ImpArray<T>`, scalars -> `T`.
fn rust_wrapper_type(ty: &Type) -> String {
    if ty.is_array() {
        format!("ImpArray<{}>", ty.basetype.rstype())
    } else {
        ty.basetype.rstype()
    }
}

/// Marshal args for a direct (single-overload) wrapper call.
/// Array args are converted to `ImpArrayRaw`; scalar args are passed through as `T`.
fn emit_marshaled_call_args(out: &mut String, args: &[Farg]) -> Vec<String> {
    let mut call_args = Vec::with_capacity(args.len());
    for arg in args {
        if is_static_array(&arg.ty) {
            out.push_str(&format!("    let {}_raw = {}.into_raw();\n", arg.id, arg.id));
            call_args.push(format!("{}_raw", arg.id));
        } else {
            call_args.push(arg.id.clone());
        }
    }
    call_args
}

/// Marshal branch arguments for a family wrapper variant.
///
/// `all_scalar_args[i]` = the family wrapper exposes arg i as plain `T`.
/// - `T` exposed, variant is scalar  -> pass through directly
/// - `ImpArray<T>` exposed, variant is scalar -> extract `.data[0]`
/// - `ImpArray<T>` exposed, variant is array  -> `.into_raw()`
fn emit_marshaled_branch_args(
    out: &mut String,
    args: &[Farg],
    branch_names: &[String],
    all_scalar_args: &[bool],
    indent: usize,
) -> Vec<String> {
    let pad = "    ".repeat(indent);
    let mut call_args = Vec::with_capacity(args.len());
    for ((arg, branch_name), &exposed_as_scalar) in
        args.iter().zip(branch_names.iter()).zip(all_scalar_args.iter())
    {
        if is_static_array(&arg.ty) {
            out.push_str(&format!(
                "{pad}let {branch_name}_raw = {branch_name}.into_raw();\n"
            ));
            call_args.push(format!("{branch_name}_raw"));
        } else if exposed_as_scalar {
            // wrapper takes T, variant expects scalar -> pass through
            call_args.push(branch_name.clone());
        } else {
            // wrapper takes ImpArray<T>, variant expects scalar -> extract element
            out.push_str(&format!(
                "{pad}let {branch_name}_val = {branch_name}.data[0];\n"
            ));
            call_args.push(format!("{branch_name}_val"));
        }
    }
    call_args
}

/// Return expression for a direct (single-overload) wrapper.
fn emit_return_expr(symbol_name: &str, ret_type: &Type, call_args: &[String]) -> String {
    if is_static_array(ret_type) {
        format!(
            "let __res_raw = unsafe {{ IMP_{}({}) }};\nunsafe {{ ImpArray::<{}>::from_raw(__res_raw) }}",
            symbol_name,
            call_args.join(", "),
            ret_type.basetype.rstype(),
        )
    } else {
        format!("unsafe {{ IMP_{}({}) }}", symbol_name, call_args.join(", "))
    }
}

/// Return expression for one variant inside a family wrapper.
///
/// `all_scalar_ret` = the family wrapper's declared return type is `T`.
fn emit_family_return_expr(
    symbol_name: &str,
    ret_type: &Type,
    call_args: &[String],
    all_scalar_ret: bool,
) -> String {
    if all_scalar_ret {
        format!("unsafe {{ IMP_{}({}) }}", symbol_name, call_args.join(", "))
    } else if is_static_array(ret_type) {
        format!(
            "let __res_raw = unsafe {{ IMP_{}({}) }};\nunsafe {{ ImpArray::<{}>::from_raw(__res_raw) }}",
            symbol_name,
            call_args.join(", "),
            ret_type.basetype.rstype(),
        )
    } else {
        // variant returns scalar but wrapper return type is ImpArray<T>
        format!(
            "let __res_val = unsafe {{ IMP_{}({}) }};\nImpArray {{ shp: vec![], data: vec![__res_val] }}",
            symbol_name,
            call_args.join(", "),
        )
    }
}

/// Build the dispatch condition for one overload variant inside a family wrapper.
///
/// Only checks positions exposed as `ImpArray<T>` (i.e., `!all_scalar_args[i]`).
/// Positions exposed as `T` are always scalar -- no runtime check needed.
fn build_variant_condition(args: &[Farg], all_scalar_args: &[bool]) -> String {
    let mut checks = Vec::new();
    let mut bound_dims: Vec<(String, String)> = Vec::new();
    let mut bound_ranks: Vec<(String, String)> = Vec::new();

    for (arg_index, (arg, &exposed_as_scalar)) in
        args.iter().zip(all_scalar_args.iter()).enumerate()
    {
        if exposed_as_scalar {
            continue;
        }
        match &arg.ty.shape {
            TypePattern::Scalar => {
                checks.push(format!("arg{arg_index}.shp.is_empty()"));
            }
            TypePattern::Axes(axes) => {
                if axes.iter().any(|ax| matches!(ax, AxisPattern::Rank(_))) {
                    checks.push(format!("!arg{arg_index}.shp.is_empty()"));
                } else {
                    checks.push(format!("arg{arg_index}.shp.len() == {}", axes.len()));
                }
                for (axis_index, axis) in axes.iter().enumerate() {
                    match axis {
                        AxisPattern::Dim(DimCapture::Known(v)) => {
                            checks.push(format!("arg{arg_index}.shp[{axis_index}] == {v}"));
                        }
                        AxisPattern::Dim(DimCapture::Var(extent)) => {
                            let expr = format!("arg{arg_index}.shp[{axis_index}]");
                            if let Some((_, bound_expr)) =
                                bound_dims.iter().find(|(name, _)| name == extent)
                            {
                                checks.push(format!("{expr} == {bound_expr}"));
                            } else {
                                bound_dims.push((extent.clone(), expr));
                            }
                        }
                        AxisPattern::Rank(RankCapture { dim: DimCapture::Var(dim), shp: _ }) => {
                            let expr = format!("arg{arg_index}.shp.len()");
                            if let Some((_, bound_expr)) = bound_ranks.iter().find(|(name, _)| name == dim)
                            {
                                checks.push(format!("{expr} == {bound_expr}"));
                            } else {
                                bound_ranks.push((dim.clone(), expr));
                            }
                        }
                        AxisPattern::Rank(_capture) => {
                            todo!()
                        }
                    }
                }
            }
        }
    }

    if checks.is_empty() {
        "true".to_owned()
    } else {
        checks.join(" && ")
    }
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
                AxisPattern::Dim(DimCapture::Known(v)) => {
                    out.push_str(&format!(
                        "    assert_eq!({}.shp[{}], {}, \"{} extent mismatch at axis {}\");\n",
                        arg.id, idx, v, arg.id, idx,
                    ));
                }
                AxisPattern::Dim(DimCapture::Var(extent)) => {
                    let binding = format!("_imp_extent_{}", extent);
                    if bound_dims.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!(
                            "    assert_eq!({}.shp[{}], {}, \"extent {} mismatch\");\n",
                            arg.id, idx, binding, extent
                        ));
                    } else {
                        out.push_str(&format!(
                            "    let {} = {}.shp[{}];\n",
                            binding, arg.id, idx
                        ));
                        bound_dims.push(binding);
                    }
                }
                AxisPattern::Rank(RankCapture { dim: DimCapture::Var(dim), shp: _ }) => {
                    let binding = format!("_imp_rank_{}", dim);
                    if bound_ranks.iter().any(|existing| existing == &binding) {
                        out.push_str(&format!("    assert_eq!({}.shp.len(), {}, \"rank {} mismatch\");\n",
                            arg.id, binding, dim));
                    } else {
                        out.push_str(&format!("    let {} = {}.shp.len();\n",
                            binding, arg.id
                        ));
                        bound_ranks.push(binding);
                    }
                }
                AxisPattern::Rank(_capture) => {
                    todo!()
                }
            }
        }
    }

    out
}
