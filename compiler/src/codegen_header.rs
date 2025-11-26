use crate::ast::*;

pub fn compile_header(fundef: &Fundef<TypedAst>) -> String {
    let mut s = String::new();

    let ret_type = match fundef.typof(fundef.ret_value) {
        Type::U32 => "u32",
        Type::Bool => "bool",
    };

    let args: Vec<String> = fundef.args.iter().map(|key| {
        let ty_str = match fundef.typof(*key) {
            Type::U32 => "u32",
            Type::Bool => "bool",
        };
        format!("{}: {}", fundef.nameof(*key), ty_str)
    }).collect();

    s.push_str("unsafe extern \"C\" {\n");
    s.push_str(&format!("    fn DSL_{}({}) -> {};\n", fundef.id, args.join(", "), ret_type));
    s.push_str("}\n\n");

    // Here we have the opportunity to add checks, dispatch to different implementations, etc.
    s.push_str(&format!("fn {}({}) -> {} {{\n", fundef.id, args.join(", "), ret_type));
    s.push_str(&format!("    unsafe {{ DSL_{}({}) }}\n", fundef.id, fundef.args.iter().map(|key| fundef.nameof(*key).to_owned()).collect::<Vec<_>>().join(", ")));
    s.push_str("}\n");

    s
}
