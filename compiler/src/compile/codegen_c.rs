use std::{collections::HashSet, convert::Infallible, mem};

use crate::{ast::*, traverse::AstPass};

pub struct CodegenContext {
    emitted: HashSet<*const Avis<TypedAst>>,
    stmts: Vec<String>,
    pub output: String,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self {
            emitted: HashSet::new(),
            stmts: Vec::new(),
            output: String::new(),
        }
    }

    pub fn finish(self) -> String {
        self.output
    }

    fn ensure_local<'ast>(&mut self, avis: &'ast Avis<TypedAst>, expr: &Expr<'ast, TypedAst>, fundef: &Fundef<'ast, TypedAst>) {
        let key = avis as *const _;
        if self.emitted.insert(key) {
            let rhs = self.render_expr(expr, fundef);
            self.stmts.push(format!("{} {} = {};", to_ctype(&avis.ty), avis.name, rhs));
        }
    }

    fn expr_for<'ast>(&mut self, id: ArgOrVar<'ast, TypedAst>, fundef: &Fundef<'ast, TypedAst>) -> String {
        if let Some(i) = fundef.arg_index(id) {
            return fundef.args[i].name.clone();
        }

        let avis = id.as_local().unwrap();
        match fundef.find_local_def(avis) {
            Some(LocalDef::Assign(expr)) => {
                self.ensure_local(avis, expr, fundef);
                avis.name.clone()
            }
            Some(LocalDef::IndexRange { .. }) | None => avis.name.clone(),
        }
    }

    fn render_expr<'ast>(&mut self, expr: &Expr<'ast, TypedAst>, fundef: &Fundef<'ast, TypedAst>) -> String {
        match expr {
            Expr::Tensor(Tensor { iv, lb, ub, ret, ssa }) => {
                let mut forloop = String::new();
                let ty = to_ctype(fundef.typof(*ret));
                let iv_name = iv.name.clone();
                let lb_name = self.expr_for(*lb, fundef);
                let ub_name = self.expr_for(*ub, fundef);

                forloop.push_str(&format!("for (size_t {} = {}; {} < {}; {} += 1) {{\n", iv_name, lb_name, iv_name, ub_name, iv_name));
                let mut scope = fundef.ssa.clone();
                scope.extend(ssa.iter().copied());
                let expr_code = self.expr_for(*ret, &Fundef {
                    name: fundef.name.clone(),
                    args: fundef.args.clone(),
                    ids: fundef.ids.clone(),
                    ssa: scope,
                    ret: *ret,
                });
                forloop.push_str(&format!("        res[{}] = {};\n", iv_name, expr_code));
                forloop.push_str("    }");
                self.stmts.push(format!("{} *res = ({} *)malloc({} * sizeof({}));", ty, ty, ub_name, ty));
                self.stmts.push(forloop);
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

impl<'ast> AstPass<'ast> for CodegenContext {
    type InAst = TypedAst;
    type OutAst = TypedAst;
    type Ok = ();
    type Err = Infallible;

    fn pass_program(&mut self, program: Program<'ast, TypedAst>) -> Result<(Self::Ok, Program<'ast, TypedAst>), Self::Err> {
        self.output.clear();
        let mut out = String::new();
        out.push_str("#include <stdlib.h>\n");
        out.push_str("#include <stdbool.h>\n");
        out.push_str("#include <stdint.h>\n");

        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let (_, fundef) = self.pass_fundef(fundef)?;
            out.push('\n');
            fundefs.push(fundef);
        }

        self.output = format!("{}{}", out, self.output);
        Ok(((), Program { fundefs }))
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, TypedAst>) -> Result<(Self::Ok, Fundef<'ast, TypedAst>), Self::Err> {
        let mut res = String::new();
        self.emitted.clear();
        let args: Vec<String> = fundef.args.iter().map(|avis| format!("{} {}", to_ctype(&avis.ty), avis.name)).collect();
        let ret_type = fundef.typof(fundef.ret);
        res.push_str(&format!("{} DSL_{}({}) {{\n", to_ctype(ret_type), fundef.name, args.join(", ")));

        let ret_code = self.expr_for(fundef.ret, &fundef);

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts {
            res.push_str(&format!("    {}\n", stmt));
        }

        res.push_str(&format!("    return {};\n", ret_code));
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
