use std::mem;

use crate::ast::*;

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
        let ret_type = to_ctype(fundef.vars[fundef.ret_value].ty);

        let args: Vec<String> = fundef.args.iter().map(|key| {
            let varinfo = &fundef.vars[*key];
            let ty_str = to_ctype(varinfo.ty);
            format!("{} {}", ty_str, varinfo.id)
        }).collect();

        c_code.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.id, args.join(", ")));

        let ret_code = self.compile_arg_or_expr(fundef, fundef.ret_value);

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts.into_iter().rev() {
            c_code.push_str(&format!("    {}\n", stmt));
        }

        c_code.push_str(&format!("    return {};\n", ret_code));

        c_code.push_str("}\n");

        c_code
    }

    pub fn compile_arg_or_expr(&mut self, fundef: &Fundef<TypedAst>, key: VarKey) -> String {
        if let Some(expr) = fundef.ssa.get(key) {
            self.compile_expr(fundef, expr)
        } else {
            fundef.vars[key].id.clone()
        }
    }

    pub fn compile_expr(&mut self, fundef: &Fundef<TypedAst>, expr: &Expr) -> String {
        let mut c_code = String::new();

        match expr {
            Expr::Binary(Binary { l, r, op }) => {
                let l_info = &fundef.vars[*l];
                let l_code = self.compile_arg_or_expr(fundef, *l);
                self.stmts.push(format!("{} {} = {};", to_ctype(l_info.ty), l_info.id, l_code));

                let r_info = &fundef.vars[*r];
                let r_code = self.compile_arg_or_expr(fundef, *r);
                self.stmts.push(format!("{} {} = {};", to_ctype(r_info.ty), r_info.id, r_code));

                c_code.push_str(&format!("{} {} {}", l_info.id, op, r_info.id));
            },
            Expr::Unary(Unary { r, op }) => {
                let r_info = &fundef.vars[*r];
                let r_code = self.compile_arg_or_expr(fundef, *r);
                self.stmts.push(format!("{} {} = {};", to_ctype(r_info.ty), r_info.id, r_code));

                c_code.push_str(&format!("{} {}", op, r_info.id));
            },
            Expr::Bool(v) => {
                c_code.push_str(&format!("{}", *v));
            },
            Expr::U32(v) => {
                c_code.push_str(&format!("{}", *v));
            },
        }

        c_code
    }
}

fn to_ctype(ty: Type) -> &'static str {
    match ty {
        Type::U32 => "uint32_t",
        Type::Bool => "bool",
    }
}
