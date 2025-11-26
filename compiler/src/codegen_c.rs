use std::mem;

use crate::ast::*;

// TODO: we probably want an undo-ssa traversal before code generation
// to share computation where possible, or push scope-local computations inside their scope
pub struct CodegenContext {
    stmts: Vec<String>,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self {
            stmts: Vec::new(),
        }
    }

    pub fn compile_program(&mut self, program: &Program<TypedAst>) -> String {
        let mut c_code = String::new();

        c_code.push_str("#include <stdbool.h>\n");
        c_code.push_str("#include <stdint.h>\n");

        for fundef in &program.fundefs {
            c_code.push_str("\n");
            c_code.push_str(&self.compile_fundef(fundef));
        }

        c_code
    }

    pub fn compile_fundef(&mut self, fundef: &Fundef<TypedAst>) -> String {
        let mut c_code = String::new();

        // Function signature
        let ret_type = to_ctype(fundef.typof(fundef.ret_value));

        let args: Vec<String> = fundef.args.iter().map(|key| {
            let ty_str = to_ctype(fundef.typof(*key));
            format!("{} {}", ty_str, fundef.nameof(*key))
        }).collect();

        c_code.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.id, args.join(", ")));

        let ret_code = self.compile_arg_or_expr(fundef, fundef.ret_value)
            .unwrap_or(fundef.nameof(fundef.ret_value).to_owned());

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts.into_iter().rev() {
            c_code.push_str(&format!("    {}\n", stmt));
        }

        c_code.push_str(&format!("    return {};\n", ret_code));

        c_code.push_str("}\n");

        c_code
    }

    pub fn compile_arg_or_expr(&mut self, fundef: &Fundef<TypedAst>, key: VarKey) -> Option<String> {
        if let Some(expr) = fundef.ssa.get(key) {
            Some(self.compile_expr(fundef, expr))
        } else {
            None
        }
    }

    pub fn compile_expr(&mut self, fundef: &Fundef<TypedAst>, expr: &Expr) -> String {
        let mut c_code = String::new();

        match expr {
            Expr::Binary(Binary { l, r, op }) => {
                if let Some(l_code) = self.compile_arg_or_expr(fundef, *l) {
                    self.stmts.push(format!("{} {} = {};", to_ctype(fundef.typof(*l)), fundef.nameof(*l), l_code));
                }

                if let Some(r_code) = self.compile_arg_or_expr(fundef, *r) {
                    self.stmts.push(format!("{} {} = {};", to_ctype(fundef.typof(*r)), fundef.nameof(*r), r_code));
                }

                c_code.push_str(&format!("{} {} {}", fundef.nameof(*l), op, fundef.nameof(*r)));
            },
            Expr::Unary(Unary { r, op }) => {
                if let Some(r_code) = self.compile_arg_or_expr(fundef, *r) {
                    self.stmts.push(format!("{} {} = {};", to_ctype(fundef.typof(*r)), fundef.nameof(*r), r_code));
                }

                c_code.push_str(&format!("{} {}", op, fundef.nameof(*r)));
            },
            Expr::Bool(v) => {
                c_code.push_str(if *v { "true" } else { "false" });
            },
            Expr::U32(v) => {
                c_code.push_str(&format!("{}", *v));
            },
        }

        c_code
    }
}

fn to_ctype(ty: &Type) -> &'static str {
    match ty {
        Type::U32 => "uint32_t",
        Type::Bool => "bool",
    }
}
