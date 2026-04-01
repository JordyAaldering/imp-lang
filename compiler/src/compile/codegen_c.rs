use std::{collections::HashSet, mem};

use crate::{ast::*, traverse::AstVisit};

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

    fn ensure_local<'ast>(
        &mut self,
        avis: &'ast Avis<TypedAst>,
        expr: &Expr<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &[ScopeBlock<'ast, TypedAst>],
    ) {
        let key = avis as *const _;
        if self.emitted.insert(key) {
            let rhs = self.render_expr(expr, fundef, extra_scopes);
            self.stmts.push(format!("{} {} = {};", to_ctype(&avis.ty), avis.name, rhs));
        }
    }

    fn body_scope<'ast>(&self, fundef: &Fundef<'ast, TypedAst>) -> ScopeBlock<'ast, TypedAst> {
        fundef.scope_block()
    }

    fn expr_for<'ast>(
        &mut self,
        id: ArgOrVar<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &[ScopeBlock<'ast, TypedAst>],
    ) -> String {
        if let Some(i) = fundef.arg_index(id) {
            return fundef.args[i].name.clone();
        }

        let Some(avis) = id.as_local() else {
            return fundef.nameof(id).to_owned();
        };

        let mut scopes = Vec::with_capacity(1 + extra_scopes.len());
        scopes.push(self.body_scope(fundef));
        scopes.extend(extra_scopes.iter().cloned());

        match find_local_in_scopes(&scopes, avis) {
            Some(LocalDef::Assign(expr)) => {
                self.ensure_local(avis, expr, fundef, extra_scopes);
                avis.name.clone()
            }
            Some(LocalDef::IndexRange { .. }) | None => avis.name.clone(),
        }
    }

    fn render_expr<'ast>(
        &mut self,
        expr: &Expr<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &[ScopeBlock<'ast, TypedAst>],
    ) -> String {
        match expr {
            Expr::Tensor(tensor) => {
                let mut forloop = String::new();
                let mut tensor_scopes = extra_scopes.to_vec();
                tensor_scopes.push(tensor.scope_block());

                let ty = to_ctype(fundef.typof(tensor.ret));
                let iv_name = tensor.iv.name.clone();
                let lb_name = self.expr_for(tensor.lb, fundef, &tensor_scopes);
                let ub_name = self.expr_for(tensor.ub, fundef, &tensor_scopes);

                let mut outer_stmts = Vec::new();
                mem::swap(&mut outer_stmts, &mut self.stmts);
                let expr_code = self.expr_for(tensor.ret, fundef, &tensor_scopes);
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
                let l = self.expr_for(*l, fundef, extra_scopes);
                let r = self.expr_for(*r, fundef, extra_scopes);
                format!("{} {} {}", l, op, r)
            }
            Expr::Unary(Unary { r, op }) => {
                let r = self.expr_for(*r, fundef, extra_scopes);
                format!("{} {}", op, r)
            }
            Expr::Bool(v) => if *v { "true".to_owned() } else { "false".to_owned() },
            Expr::U32(v) => format!("{}", *v),
        }
    }
}

impl<'ast> AstVisit<'ast> for CodegenContext {
    type Ast = TypedAst;

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

        let ret_code = self.expr_for(ret, &fundef, &[]);

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
