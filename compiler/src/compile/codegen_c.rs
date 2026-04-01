use std::mem;

use crate::{ast::*, traverse::Traverse};

pub struct CodegenContext {
    stmts: Vec<String>,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self { stmts: Vec::new() }
    }

    fn expr_for<'ast>(&mut self, id: ArgOrVar<'ast, TypedAst>, fundef: &Fundef<'ast, TypedAst>) -> String {
        match id {
            ArgOrVar::Arg(i) => fundef.args[i].name.clone(),
            ArgOrVar::Var(v) => self.trav_expr(&mut fundef.find_ssa(v).unwrap().clone(), fundef),
            ArgOrVar::Iv(v) => v.name.clone(),
        }
    }

    fn trav_expr<'ast>(&mut self, expr: &mut Expr<'ast, TypedAst>, fundef: &Fundef<'ast, TypedAst>) -> String {
        match expr {
            Expr::Tensor(Tensor { iv, lb, ub, ret, ssa }) => {
                let mut forloop = String::new();
                let ty = to_ctype(fundef.typof(*ret));
                let iv_name = iv.name.clone();
                let lb_name = fundef.nameof(*lb).to_owned();
                let ub_name = fundef.nameof(*ub).to_owned();

                forloop.push_str(&format!("for (size_t {} = {}; {} < {}; {} += 1) {{\n", iv_name, lb_name, iv_name, ub_name, iv_name));
                let expr_code = self.expr_for(*ret, &Fundef {
                    name: fundef.name.clone(),
                    args: fundef.args.clone(),
                    ids: fundef.ids.clone(),
                    ssa: ssa.clone(),
                    ret: *ret,
                });
                forloop.push_str(&format!("        res[{}] = {};\n", iv_name, expr_code));
                forloop.push_str("    }");
                self.stmts.insert(0, forloop);
                self.stmts.push(format!("{} *res = ({} *)malloc({} * sizeof({}));", ty, ty, ub_name, ty));
                "res".to_owned()
            }
            Expr::Binary(Binary { l, r, op }) => {
                let l = self.expr_for(*l, fundef);
                let r = self.expr_for(*r, fundef);
                format!("{} {} {}", l, op, r)
            }
            Expr::Unary(Unary { r, op }) => {
                let r = self.expr_for(*r, fundef);
                format!("{} {}", op, r)
            }
            Expr::Bool(v) => if *v { "true".to_owned() } else { "false".to_owned() },
            Expr::U32(v) => format!("{}", *v),
        }
    }
}

impl<'ast> Traverse<'ast, TypedAst> for CodegenContext {
    type Output = String;
    const DEFAULT: String = String::new();

    fn trav_program(&mut self, program: &mut Program<'ast, TypedAst>) -> String {
        let mut res = String::new();
        res.push_str("#include <stdlib.h>\n");
        res.push_str("#include <stdbool.h>\n");
        res.push_str("#include <stdint.h>\n");

        for fundef in &mut program.fundefs {
            res.push('\n');
            res.push_str(&self.trav_fundef(fundef));
        }
        res
    }

    fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, TypedAst>) -> String {
        let mut res = String::new();
        let args: Vec<String> = fundef.args.iter().map(|avis| format!("{} {}", to_ctype(&avis.ty), avis.name)).collect();
        let ret_type = fundef.typof(fundef.ret);
        res.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.name, args.join(", ")));

        let ret_code = self.expr_for(fundef.ret, fundef);

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts.into_iter().rev() {
            res.push_str(&format!("    {}\n", stmt));
        }

        res.push_str(&format!("    return {};\n", ret_code));
        res.push_str("}\n");
        res
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
