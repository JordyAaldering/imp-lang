use crate::{ast::*, traverse::AstPass};

/// Rust header generation pass using AstPass traversal.
///
/// Emits Rust extern FFI bindings and wrappers for TypedAst functions.
/// For each DSL function, generates:
/// - unsafe extern "C" binding to compiled C symbol
/// - safe Rust wrapper that calls the FFI function
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

    fn pass_program(&mut self, program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
        self.output.clear();
        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.pass_fundef(fundef);
            fundefs.push(fundef);
        }

        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, TypedAst>) -> Fundef<'ast, TypedAst> {
        let mut res = String::new();

        let ret_type = to_rusttype(fundef.typof(fundef.ret_id()));

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
        fundef
    }

    fn pass_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        expr
    }

    fn pass_ssa(&mut self, id: ArgOrVar<'ast, TypedAst>) -> ArgOrVar<'ast, TypedAst> {
        id
    }

    fn pass_tensor(&mut self, tensor: Tensor<'ast, TypedAst>) -> Tensor<'ast, TypedAst> {
        tensor
    }

    fn pass_binary(&mut self, binary: Binary<'ast, TypedAst>) -> Binary<'ast, TypedAst> {
        binary
    }

    fn pass_unary(&mut self, unary: Unary<'ast, TypedAst>) -> Unary<'ast, TypedAst> {
        unary
    }

    fn pass_bool(&mut self, value: bool) -> bool {
        value
    }

    fn pass_u32(&mut self, value: u32) -> u32 {
        value
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
