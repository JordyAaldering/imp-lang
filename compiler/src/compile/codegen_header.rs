use std::convert::Infallible;

use crate::{ast::*, traverse::AstPass};

pub struct CompileHeader {
    output: String,
}

impl CompileHeader {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn finish(self) -> String {
        self.output
    }
}

impl<'ast> AstPass<'ast> for CompileHeader {
    type InAst = TypedAst;
    type OutAst = TypedAst;
    type Ok = ();
    type Err = Infallible;

    fn pass_program(&mut self, program: Program<'ast, TypedAst>) -> Result<(Self::Ok, Program<'ast, TypedAst>), Self::Err> {
        self.output.clear();
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let (_, fundef) = self.pass_fundef(fundef)?;
            fundefs.push(fundef);
        }

        Ok(((), Program { fundefs }))
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, TypedAst>) -> Result<(Self::Ok, Fundef<'ast, TypedAst>), Self::Err> {
        let mut res = String::new();

        let ret_type = to_rusttype(fundef.typof(fundef.ret));

        let args = fundef.args.iter()
            .map(|arg| format!("{}: {}", arg.name, to_rusttype(&arg.ty)))
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

        self.output.push_str(&res);
        Ok(((), fundef))
    }

    fn pass_ssa(&mut self, id: ArgOrVar<'ast, TypedAst>) -> Result<(Self::Ok, ArgOrVar<'ast, TypedAst>), Self::Err> {
        Ok(((), id))
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, TypedAst>) -> Result<(Self::Ok, Tensor<'ast, TypedAst>), Self::Err> {
        Ok(((), tensor))
    }

    fn pass_binary(&mut self, binary: Binary<'ast, TypedAst>) -> Result<(Self::Ok, Binary<'ast, TypedAst>), Self::Err> {
        Ok(((), binary))
    }

    fn pass_unary(&mut self, unary: Unary<'ast, TypedAst>) -> Result<(Self::Ok, Unary<'ast, TypedAst>), Self::Err> {
        Ok(((), unary))
    }

    fn pass_bool(&mut self, value: bool) -> Result<(Self::Ok, bool), Self::Err> {
        Ok(((), value))
    }

    fn pass_u32(&mut self, value: u32) -> Result<(Self::Ok, u32), Self::Err> {
        Ok(((), value))
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
