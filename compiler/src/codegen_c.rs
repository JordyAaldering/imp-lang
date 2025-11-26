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

    pub fn compile_program(&mut self, program: &Program) -> String {
        let mut c_code = String::new();

        c_code.push_str("#include <stdbool.h>\n");
        c_code.push_str("#include <stdint.h>\n");

        for fundef in &program.fundefs {
            c_code.push_str("\n");
            c_code.push_str(&self.compile_fundef(fundef));
        }

        c_code
    }

    fn compile_fundef(&mut self, fundef: &Fundef) -> String {
        let mut c_code = String::new();

        // Function signature
        let ret_type = to_ctype(fundef[fundef.ret_id].ty);

        let args: Vec<String> = fundef.args.iter().map(|avis| {
            let ty_str = to_ctype(avis.ty);
            format!("{} {}", ty_str, avis.id)
        }).collect();

        c_code.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.id, args.join(", ")));

        let ret_code = match fundef.ret_id {
            ArgOrVar::Arg(i) => fundef.args[i].id.to_owned(),
            ArgOrVar::Var(k) => self.compile_expr(fundef, &fundef.ssa[k]),
        };

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts.into_iter().rev() {
            c_code.push_str(&format!("    {}\n", stmt));
        }

        c_code.push_str(&format!("    return {};\n", ret_code));

        c_code.push_str("}\n");

        c_code
    }

    fn compile_expr(&mut self, fundef: &Fundef, expr: &Expr) -> String {
        let mut c_code = String::new();

        match expr {
            Expr::Binary(Binary { l, r, op }) => {
                if let ArgOrVar::Var(k) = l {
                    let l_code = self.compile_expr(fundef, &fundef.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(fundef[*k].ty), fundef[*k].id, l_code));
                }

                c_code.push_str(&format!("{} {} {}", fundef[*l].id, op, fundef[*r].id));
            },
            Expr::Unary(Unary { r, op }) => {
                if let ArgOrVar::Var(k) = r {
                    let r_code = self.compile_expr(fundef, &fundef.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(fundef[*k].ty), fundef[*k].id, r_code));
                }

                c_code.push_str(&format!("{} {}", op, fundef[*r].id));
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

fn to_ctype(ty: Option<Type>) -> &'static str {
    match ty.unwrap() {
        Type::U32 => "uint32_t",
        Type::Bool => "bool",
    }
}
