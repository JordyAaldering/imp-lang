use std::mem;

use slotmap::{SecondaryMap, SlotMap};

use crate::{ast::*, traverse::Traverse};

// TODO: we probably want an undo-ssa traversal before code generation
// to share computation where possible, or push scope-local computations inside their scope
pub struct CodegenContext {
    args: Vec<Avis<TypedAst>>,
    ids: SlotMap<TypedKey, Avis<TypedAst>>,
    scopes: Vec<SecondaryMap<TypedKey, Expr<TypedAst>>>,
    stmts: Vec<String>,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            ids: SlotMap::with_key(),
            scopes: Vec::new(),
            stmts: Vec::new(),
        }
    }

    fn find(&self, key: ArgOrVar<TypedAst>) -> &Avis<TypedAst> {
        match key {
            ArgOrVar::Arg(i) => &self.args[i],
            ArgOrVar::Var(k) => &self.ids[k],
            ArgOrVar::Iv(k) => &self.ids[k],
        }
    }

    fn find_ssa(&self, key: TypedKey) -> &Expr<TypedAst> {
        for scope in self.scopes.iter().rev() {
            if let Some(expr) = scope.get(key) {
                return expr;
            }
        }
        unreachable!()
    }
}

impl Traverse<TypedAst> for CodegenContext {
    type Output = String;

    const DEFAULT: String = String::new();

    fn trav_program(&mut self, program: &Program<TypedAst>) -> String {
        let mut res = String::new();

        res.push_str("#include <stdlib.h>\n");
        res.push_str("#include <stdbool.h>\n");
        res.push_str("#include <stdint.h>\n");

        for fundef in &program.fundefs {
            res.push('\n');
            res.push_str(&self.trav_fundef(fundef));
        }

        res
    }

    fn trav_fundef(&mut self, fundef: &Fundef<TypedAst>) -> String {
        self.args = fundef.args.clone();
        self.ids = fundef.ids.clone();
        self.scopes.push(fundef.ssa.clone());
        let mut res = String::new();

        // Function signature
        let ret_type = fundef.typof(fundef.ret);

        let args: Vec<String> = fundef.args.iter().map(|avis| {
            let ty_str = to_ctype(&avis.ty);
            format!("{} {}", ty_str, avis.name)
        }).collect();

        res.push_str(&format!("{} DSL_{}({}) {{\n", ret_type, fundef.name, args.join(", ")));

        let ret_code = match fundef.ret {
            ArgOrVar::Arg(i) => fundef.args[i].name.to_owned(),
            ArgOrVar::Var(k) => self.trav_expr(&fundef.ssa[k]),
            ArgOrVar::Iv(_) => unreachable!(),
        };

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts.into_iter().rev() {
            res.push_str(&format!("    {}\n", stmt));
        }

        res.push_str(&format!("    return {};\n", ret_code));

        res.push_str("}\n");

        self.scopes.pop().unwrap();
        assert!(self.scopes.is_empty());
        res
    }

    fn trav_expr(&mut self, expr: &Expr<TypedAst>) -> String {
        let mut res = String::new();

        match expr {
            Expr::Tensor(Tensor { iv, lb, ub, ret, ssa }) => {
                self.scopes.push(ssa.clone());

                let mut forloop = String::new();

                let ty = to_ctype(&self.find(*ret).ty);
                let iv_name = self.ids[*iv].name.clone();
                let lb_name = self.find(*lb).name.clone();
                let ub_name = self.find(*ub).name.clone();

                forloop.push_str(&format!("for (size_t {} = {}; {} < {}; {} += 1) {{\n", iv_name, lb_name, iv_name, ub_name, iv_name));

                if let ArgOrVar::Var(k) = ret {
                    let expr_code = self.trav_expr(&self.find_ssa(*k).clone());
                    forloop.push_str(&format!("        res[{}] = {};\n", iv_name, expr_code));
                }

                forloop.push_str("    }");
                self.stmts.insert(0, forloop);

                self.stmts.push(format!("{} *res = ({} *)malloc({} * sizeof({}));", ty, ty, ub_name, ty));

                if let ArgOrVar::Var(k) = ub {
                    let l_code = self.trav_expr(&self.find_ssa(*k).clone());
                    self.stmts.push(format!("{} {} = {};", to_ctype(&self.ids[*k].ty), self.ids[*k].name, l_code));
                }

                if let ArgOrVar::Var(k) = lb {
                    let l_code = self.trav_expr(&self.find_ssa(*k).clone());
                    self.stmts.push(format!("{} {} = {};", to_ctype(&self.ids[*k].ty), self.ids[*k].name, l_code));
                }

                res.push_str("res");
                self.scopes.pop().unwrap();
            }
            Expr::Binary(Binary { l, r, op }) => {
                if let ArgOrVar::Var(k) = l {
                    let l_code = self.trav_expr(&self.find_ssa(*k).clone());
                    self.stmts.push(format!("{} {} = {};", to_ctype(&self.ids[*k].ty), self.ids[*k].name, l_code));
                }

                if let ArgOrVar::Var(k) = r {
                    let r_code = self.trav_expr(&self.find_ssa(*k).clone());
                    self.stmts.push(format!("{} {} = {};", to_ctype(&self.ids[*k].ty), self.ids[*k].name, r_code));
                }

                res.push_str(&format!("{} {} {}", self.find(*l).name, op, self.find(*r).name));
            },
            Expr::Unary(Unary { r, op }) => {
                if let ArgOrVar::Var(k) = r {
                    let r_code = self.trav_expr(&self.find_ssa(*k).clone());
                    self.stmts.push(format!("{} {} = {};", to_ctype(&self.ids[*k].ty), self.ids[*k].name, r_code));
                }

                res.push_str(&format!("{} {}", op, self.find(*r).name));
            },
            Expr::Bool(v) => {
                res.push_str(if *v { "true" } else { "false" });
            },
            Expr::U32(v) => {
                res.push_str(&format!("{}", *v));
            },
        }

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
