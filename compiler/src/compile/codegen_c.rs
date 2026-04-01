use std::{collections::HashSet, mem};

use crate::{ast::*, traverse::AstPass};

/// C code generation pass using AstPass traversal.
///
/// Emits C99 code for a TypedAst program. Tracks which locals have been emitted
/// to avoid re-emission. Expression rendering handles tensor loops by temporarily
/// extending the function scope.
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

        let Some(avis) = id.as_local() else {
            return fundef.nameof(id).to_owned();
        };
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
                let mut scope = fundef.body.clone();
                scope.extend(ssa.iter().copied());
                scope.push(Stmt::Return { id: *ret });
                let tensor_fundef = Fundef {
                    name: fundef.name.clone(),
                    args: fundef.args.clone(),
                    ids: fundef.ids.clone(),
                    body: scope,
                };

                let ty = to_ctype(tensor_fundef.typof(*ret));
                let iv_name = iv.name.clone();
                let lb_name = self.expr_for(*lb, &tensor_fundef);
                let ub_name = self.expr_for(*ub, &tensor_fundef);

                let mut outer_stmts = Vec::new();
                mem::swap(&mut outer_stmts, &mut self.stmts);
                let expr_code = self.expr_for(*ret, &tensor_fundef);
                let mut body_stmts = Vec::new();
                mem::swap(&mut body_stmts, &mut self.stmts);
                self.stmts = outer_stmts;

                forloop.push_str(&format!("for (size_t {} = {}; {} < {}; {} += 1) {{\n", iv_name, lb_name, iv_name, ub_name, iv_name));
                for stmt in body_stmts {
                    forloop.push_str(&format!("        {}\n", stmt));
                }
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

    fn pass_program(&mut self, program: Program<'ast, TypedAst>) -> Program<'ast, TypedAst> {
        self.output.clear();
        let mut out = String::new();
        out.push_str("#include <stdlib.h>\n");
        out.push_str("#include <stdbool.h>\n");
        out.push_str("#include <stdint.h>\n");

        let mut fundefs = Vec::with_capacity(program.fundefs.len());
        for fundef in program.fundefs {
            let fundef = self.pass_fundef(fundef);
            out.push('\n');
            fundefs.push(fundef);
        }

        self.output = format!("{}{}", out, self.output);
        Program { fundefs }
    }

    fn pass_fundef(&mut self, fundef: Fundef<'ast, TypedAst>) -> Fundef<'ast, TypedAst> {
        let mut res = String::new();
        self.emitted.clear();
        let args: Vec<String> = fundef.args.iter().map(|avis| format!("{} {}", to_ctype(&avis.ty), avis.name)).collect();
        let ret = fundef.ret_id();
        let ret_type = fundef.typof(ret);
        res.push_str(&format!("{} DSL_{}({}) {{\n", to_ctype(ret_type), fundef.name, args.join(", ")));

        let ret_code = self.expr_for(ret, &fundef);

        let mut stmts = Vec::new();
        mem::swap(&mut stmts, &mut self.stmts);
        for stmt in stmts {
            res.push_str(&format!("    {}\n", stmt));
        }

        res.push_str(&format!("    return {};\n", ret_code));
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
