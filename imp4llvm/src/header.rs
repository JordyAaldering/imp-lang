use compiler::ast::*;

pub fn compile_header(fundef: &Fundef<TypedAst>) -> String {
    let mut s = String::new();

    let ret_type = match fundef.vars[fundef.ret_value].ty {
        Type::U32 => "u32",
        Type::Bool => "bool",
    };

    let args: Vec<String> = fundef.args.iter().map(|key| {
        let vinfo = &fundef.vars[*key];
        let ty_str = match vinfo.ty {
            Type::U32 => "u32",
            Type::Bool => "bool",
        };
        format!("{}: {}", vinfo.id, ty_str)
    }).collect();

    s.push_str("unsafe extern \"C\" {\n");
    s.push_str(&format!("    fn DSL_{}({}) -> {};\n", fundef.id, args.join(", "), ret_type));
    s.push_str("}\n\n");

    // Here we have the opportunity to add checks, dispatch to different implementations, etc.
    s.push_str(&format!("fn {}({}) -> {} {{\n", fundef.id, args.join(", "), ret_type));
    s.push_str(&format!("    unsafe {{ DSL_{}({}) }}\n", fundef.id, fundef.args.iter().map(|key| fundef.vars[*key].id.clone()).collect::<Vec<_>>().join(", ")));
    s.push_str("}\n");

    s
}
