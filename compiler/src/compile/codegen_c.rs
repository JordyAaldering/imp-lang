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

        c_code.push_str("#include <stdlib.h>\n");
        c_code.push_str("#include <stdbool.h>\n");
        c_code.push_str("#include <stdint.h>\n");

        for fundef in &program.fundefs {
            c_code.push_str("\n");
            c_code.push_str(&self.compile_fundef(fundef));
        }

        c_code
    }

    fn compile_fundef(&mut self, fundef: &Fundef<TypedAst>) -> String {
        let mut c_code = String::new();

        // Function signature
        let ret_type = to_ctype(&fundef[fundef.body.ret.clone()].ty);

        let args: Vec<String> = fundef.args.iter().map(|avis| {
            let ty_str = to_ctype(&avis.ty);
            format!("{} {}", ty_str, avis.name)
        }).collect();

        c_code.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.name, args.join(", ")));

        let ret_code = match fundef.body.ret {
            ArgOrVar::Arg(i) => fundef.args[i].name.to_owned(),
            ArgOrVar::Var(k) => self.compile_expr(fundef, &fundef.body.ssa[k]),
            ArgOrVar::Iv(_) => unreachable!(),
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

    fn compile_expr(&mut self, fundef: &Fundef<TypedAst>, expr: &Expr<TypedAst>) -> String {
        let mut c_code = String::new();

        match expr {
            Expr::Tensor(Tensor { iv, body: expr, lb, ub }) => {
                let mut forloop = String::new();

                let ty = to_ctype(&fundef[expr.ret.clone()].ty);
                let iv_name = fundef.body.ids[iv.0].name.clone();
                let lb_name = fundef[lb.clone()].name.clone();
                let ub_name = fundef[ub.clone()].name.clone();

                forloop.push_str(&format!("for (size_t {} = {}; {} < {}; {} += 1) {{\n", iv_name, lb_name, iv_name, ub_name, iv_name));

                if let ArgOrVar::Var(k) = expr.ret {
                    let expr_code = self.compile_expr(fundef, &fundef.body.ssa[k]);
                    forloop.push_str(&format!("        res[{}] = {};\n", iv_name, expr_code));
                }

                forloop.push_str("    }");
                self.stmts.insert(0, forloop);

                self.stmts.push(format!("{} *res = ({} *)malloc({} * sizeof({}));", ty, ty, ub_name, ty));

                if let ArgOrVar::Var(k) = ub {
                    let l_code = self.compile_expr(fundef, &fundef.body.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(&fundef.body.ids[*k].ty), fundef.body.ids[*k].name, l_code));
                }

                if let ArgOrVar::Var(k) = lb {
                    let l_code = self.compile_expr(fundef, &fundef.body.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(&fundef.body.ids[*k].ty), fundef.body.ids[*k].name, l_code));
                }

                c_code.push_str("res");
            }
            Expr::Binary(Binary { l, r, op }) => {
                if let ArgOrVar::Var(k) = l {
                    let l_code = self.compile_expr(fundef, &fundef.body.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(&fundef.body.ids[*k].ty), fundef.body.ids[*k].name, l_code));
                }

                if let ArgOrVar::Var(k) = r {
                    let r_code = self.compile_expr(fundef, &fundef.body.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(&fundef.body.ids[*k].ty), fundef.body.ids[*k].name, r_code));
                }

                c_code.push_str(&format!("{} {} {}", fundef[l.clone()].name, op, fundef[r.clone()].name));
            },
            Expr::Unary(Unary { r, op }) => {
                if let ArgOrVar::Var(k) = r {
                    let r_code = self.compile_expr(fundef, &fundef.body.ssa[*k]);
                    self.stmts.push(format!("{} {} = {};", to_ctype(&fundef.body.ids[*k].ty), fundef.body.ids[*k].name, r_code));
                }

                c_code.push_str(&format!("{} {}", op, fundef[r.clone()].name));
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

fn to_ctype(ty: &Type) -> String {
    let base = match ty.basetype {
        BaseType::U32 => "uint32_t",
        BaseType::Bool => "bool",
    };

    let shp = match ty.shp {
        Shape::Scalar => "",
        Shape::Vector(_) => "*",
    };

    format!("{}{}", base, shp)
}
