use crate::{ast::*, traverse::Traverse};

pub struct CompileHeader;

impl CompileHeader {
    pub fn new() -> Self {
        Self
    }
}

impl Traverse<TypedAst> for CompileHeader {
    type Output = String;

    const DEFAULT: Self::Output = String::new();

    fn trav_fundef(&mut self, fundef: &mut Fundef<TypedAst>) -> Self::Output {
        let mut res = String::new();

        let ret_type = match fundef.ret {
            ArgOrVar::Arg(i) => to_rusttype(&fundef.args[i].ty),
            ArgOrVar::Var(k) => to_rusttype(&fundef.ids[k].ty),
            ArgOrVar::Iv(k) => to_rusttype(&fundef.ids[k].ty),
        };

        let args = fundef.args.iter_mut()
            .map(|arg| {
                self.trav_arg(arg)
            })
            .collect::<Vec<String>>()
            .join(", ");

        res.push_str("unsafe extern \"C\" {\n");
        res.push_str(&format!("    fn DSL_{}({}) -> {};\n", fundef.name, args, ret_type));
        res.push_str("}\n\n");

        // Here we have the opportunity to add checks, dispatch to different implementations, etc.
        res.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args, ret_type));
        res.push_str(&format!("    unsafe {{ DSL_{}({}) }}\n",
                                fundef.name,
                                fundef.args.iter().map(|avis| avis.name.to_owned())
                            .collect::<Vec<_>>().join(", ")));
        res.push_str("}\n");

        res
    }

    fn trav_arg(&mut self, arg: &mut Avis<TypedAst>) -> Self::Output {
        format!("{}: {}", arg.name, to_rusttype(&arg.ty))
    }
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
