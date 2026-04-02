use std::{collections::HashSet, mem};

use crate::{ast::*, Visit};

pub struct CompileC {
    emitted: HashSet<*const ()>,
    stmts: Vec<String>,
    output: String,
}

impl CompileC {
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
        lvis: &'ast Lvis<'ast, TypedAst>,
        expr: &Expr<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &Vec<ScopeBlock<'ast, TypedAst>>,
    ) {
        let key = lvis as *const _ as *const ();
        if self.emitted.insert(key) {
            let rhs = self.render_expr(expr, fundef, extra_scopes);
            self.stmts.push(format!("{} {} = {};", to_ctype(&lvis.ty), lvis.name, rhs));
        }
    }

    fn body_scope<'ast>(&self, fundef: &Fundef<'ast, TypedAst>) -> ScopeBlock<'ast, TypedAst> {
        fundef.scope_block()
    }

    fn expr_for<'ast>(
        &mut self,
        id: Id<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &Vec<ScopeBlock<'ast, TypedAst>>,
    ) -> String {
        if let Some(i) = fundef.arg_index(id.clone()) {
            return fundef.args[i].name.clone();
        }

        let lvis = match id {
            Id::Arg(_) => return fundef.nameof(&id),
            Id::Var(lvis) => lvis,
        };

        let mut scopes = Vec::with_capacity(1 + extra_scopes.len());
        scopes.push(self.body_scope(fundef));
        scopes.extend(extra_scopes.iter().cloned());

        match find_local_in_scopes(&scopes, lvis) {
            Some(LocalDef::Assign(expr)) => {
                self.ensure_local(lvis, expr, fundef, extra_scopes);
                lvis.name.clone()
            }
            Some(LocalDef::IndexRange { .. }) | None => lvis.name.clone(),
        }
    }

    fn render_expr<'ast>(
        &mut self,
        expr: &Expr<'ast, TypedAst>,
        fundef: &Fundef<'ast, TypedAst>,
        extra_scopes: &Vec<ScopeBlock<'ast, TypedAst>>,
    ) -> String {
        match expr {
            Expr::Tensor(tensor) => {
                let mut forloop = String::new();
                let mut tensor_scopes = extra_scopes.to_vec();
                tensor_scopes.last_mut().get_or_insert(&mut vec![]).push(
                    ScopeEntry::IndexRange {
                        iv: tensor.iv,
                        lb: tensor.lb.clone().into(),
                        ub: tensor.ub.clone().into(),
                    });
                tensor_scopes.push(tensor.build_scope());

                let ty = to_ctype(fundef.typof(&tensor.ret));
                let iv_name = tensor.iv.name.clone();
                let lb_name = self.expr_for(tensor.lb.clone(), fundef, &tensor_scopes);
                let ub_name = self.expr_for(tensor.ub.clone(), fundef, &tensor_scopes);

                let mut outer_stmts = Vec::new();
                mem::swap(&mut outer_stmts, &mut self.stmts);
                let expr_code = self.expr_for(tensor.ret.clone(), fundef, &tensor_scopes);
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
                let l = self.expr_for(l.clone(), fundef, extra_scopes);
                let r = self.expr_for(r.clone(), fundef, extra_scopes);
                format!("{} {} {}", l, op, r)
            }
            Expr::Unary(Unary { r, op }) => {
                let r = self.expr_for(r.clone(), fundef, extra_scopes);
                format!("{} {}", op, r)
            }
            Expr::Id(id) => self.expr_for(id.clone(), fundef, extra_scopes),
            Expr::Bool(v) => if *v { "true".to_owned() } else { "false".to_owned() },
            Expr::U32(v) => format!("{}", *v),
        }
    }
}

impl<'ast> Visit<'ast> for CompileC {
    type Ast = TypedAst;

    fn visit_program(&mut self, program: &Program<'ast, TypedAst>) {

        self.output.push_str("#include <stdlib.h>\n");
        self.output.push_str("#include <stdbool.h>\n");
        self.output.push_str("#include <stdint.h>\n");
        self.output.push('\n');

        for fundef in &program.fundefs {
            self.visit_fundef(fundef);
            self.output.push('\n');
        }
    }

    fn visit_fundef(&mut self, fundef: &Fundef<'ast, TypedAst>) {
        let mut res = String::new();

        self.emitted.clear();

        let args: Vec<String> = fundef.args.iter().map(|avis| format!("{} {}", to_ctype(&avis.ty), avis.name)).collect();
        let ret = fundef.ret_id();
        res.push_str(&format!("{} IMP_{}({}) {{\n", to_ctype(&fundef.ret_type), fundef.name, args.join(", ")));

        let ret_code = self.expr_for(ret, &fundef, &Vec::new());

        for stmt in &self.stmts {
            res.push_str(&format!("    {}\n", stmt));
        }

        res.push_str(&format!("    return {};\n", ret_code));
        res.push_str("}\n");

        self.output.push_str(&res);
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
