use crate::ast::*;

pub fn compile_header(fundef: &Fundef<TypedAst>) -> String {
    let mut s = String::new();

    let ret_type = to_rusttype(&fundef[fundef.ret.clone()].ty);

    let args: Vec<String> = fundef.args.iter().map(|avis| {
        let ty_str = to_rusttype(&avis.ty);
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

fn to_rusttype(ty: &Type) -> String {
    let base = match ty.basetype {
        BaseType::U32 => "u32",
        BaseType::Bool => "bool",
    };

    match ty.shp {
        Shape::Scalar => base.to_owned(),
        Shape::Vector(_) => format!("Vec<{}>", base),
    }
}
