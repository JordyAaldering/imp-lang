use crate::{ast::*, traverse::Traversal};

pub struct CompileHeader {
    pub header: String,
}

impl CompileHeader {
    pub fn new() -> Self {
        Self { header: String::new() }
    }
}

impl Traversal<TypedAst> for CompileHeader {
    type Err = ();

    fn trav_fundef(&mut self, fundef: Fundef<TypedAst>) -> Result<Fundef<TypedAst>, Self::Err> {
        let ret_type = to_rusttype(&fundef[fundef.block.ret.clone()].ty);

        let args: Vec<String> = fundef.args.iter().map(|avis| {
            let ty_str = to_rusttype(&avis.ty);
            format!("{}: {}", avis.name, ty_str)
        }).collect();

        self.header.push_str("unsafe extern \"C\" {\n");
        self.header.push_str(&format!("    fn DSL_{}({}) -> {};\n", fundef.name, args.join(", "), ret_type));
        self.header.push_str("}\n\n");

        // Here we have the opportunity to add checks, dispatch to different implementations, etc.
        self.header.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args.join(", "), ret_type));
        self.header.push_str(&format!("    unsafe {{ DSL_{}({}) }}\n",
                                fundef.name,
                                fundef.args.iter().map(|avis| avis.name.to_owned())
                            .collect::<Vec<_>>().join(", ")));
        self.header.push_str("}\n");

        Ok(fundef)
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
