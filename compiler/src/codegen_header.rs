use crate::ast::*;

pub fn compile_header(fundef: &Fundef) -> String {
    let mut s = String::new();

    let ret_type = match fundef[fundef.ret_id].ty.unwrap() {
        Type::U32 => "u32",
        Type::Bool => "bool",
    };

    let args: Vec<String> = fundef.args.iter().map(|avis| {
        let ty_str = match avis.ty.unwrap() {
            Type::U32 => "u32",
            Type::Bool => "bool",
        };
        format!("{}: {}", avis.name, ty_str)
    }).collect();

    s.push_str("unsafe extern \"C\" {\n");
    s.push_str(&format!("    fn DSL_{}({}) -> {};\n", fundef.name, args.join(", "), ret_type));
    s.push_str("}\n\n");

    // Here we have the opportunity to add checks, dispatch to different implementations, etc.
    s.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args.join(", "), ret_type));
    s.push_str(&format!("    unsafe {{ DSL_{}({}) }}\n", fundef.name, fundef.args.iter().map(|avis| avis.name.to_owned()).collect::<Vec<_>>().join(", ")));
    s.push_str("}\n");

    s
}
