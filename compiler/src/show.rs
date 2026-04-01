use crate::ast::*;

pub fn show<'ast, Ast: AstConfig>(program: &Program<'ast, Ast>) -> String {
    program.fundefs.iter().map(show_fundef).collect::<Vec<_>>().join("\n\n")
}

fn show_fundef<'ast, Ast: AstConfig>(fundef: &Fundef<'ast, Ast>) -> String {
    let mut out = String::new();
    let args = fundef.args.iter().map(|arg| format!("{} {}", arg.ty, arg.name)).collect::<Vec<_>>().join(", ");
    out.push_str(&format!("fn {}({}) -> {} {{\n", fundef.name, args, fundef.typof(fundef.ret)));

    for id in &fundef.ids {
        out.push_str(&format!("    {} {};\n", id.ty, id.name));
    }

    for entry in &fundef.ssa {
        if let ScopeEntry::Assign { avis, expr } = entry {
            out.push_str(&format!("    {} = {};\n", avis.name, show_expr(expr, 1, &fundef.args)));
        }
    }

    out.push_str(&format!("    return {};\n", fundef.nameof(fundef.ret)));
    out.push_str("}");
    out
}

fn show_expr<'ast, Ast: AstConfig>(expr: &Expr<'ast, Ast>, level: usize, args: &[&'ast Avis<Ast>]) -> String {
    match expr {
        Expr::Tensor(t) => {
            let mut out = String::new();
            let indent = " ".repeat(4 * level);
            out.push_str("{\n");
            for entry in &t.ssa {
                if let ScopeEntry::Assign { avis, expr } = entry {
                    out.push_str(&format!("{}{} = {};\n", indent, avis.name, show_expr(expr, level + 1, args)));
                }
            }
            out.push_str(&format!("{}return {};\n", indent, name_of(t.ret, args)));
            out.push_str(&format!("{}| {} <= {} < {} }}", indent, name_of(t.lb, args), t.iv.name, name_of(t.ub, args)));
            out
        }
        Expr::Binary(b) => format!("{} {} {}", name_of(b.l, args), b.op, name_of(b.r, args)),
        Expr::Unary(u) => format!("{} {}", u.op, name_of(u.r, args)),
        Expr::Bool(v) => v.to_string(),
        Expr::U32(v) => v.to_string(),
    }
}

fn name_of<'ast, Ast: AstConfig>(id: ArgOrVar<'ast, Ast>, args: &[&'ast Avis<Ast>]) -> String {
    match id {
        ArgOrVar::Arg(i) => args[i].name.clone(),
        ArgOrVar::Var(v) => v.name.clone(),
    }
}
