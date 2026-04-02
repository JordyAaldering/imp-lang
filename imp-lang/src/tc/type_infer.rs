use std::collections::HashMap;

use crate::{ast::*, traverse::Traverse};

pub fn type_infer<'ast>(program: Program<'ast, UntypedAst>) -> Result<Program<'ast, TypedAst>, InferenceError> {
    Ok(TypeInfer::new().trav_program(program))
}

pub struct TypeInfer<'ast> {
    args: Vec<&'ast Farg>,
    idmap: HashMap<*const VarInfo<'ast, UntypedAst>, &'ast VarInfo<'ast, TypedAst>>,
    new_ids: Vec<&'ast VarInfo<'ast, TypedAst>>,
}

#[derive(Debug)]
pub enum InferenceError {}

impl<'ast> TypeInfer<'ast> {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            idmap: HashMap::new(),
            new_ids: Vec::new(),
        }
    }

    fn alloc_farg(&self, name: String, ty: Type) -> &'ast Farg {
        Box::leak(Box::new(Farg { name, ty }))
    }

    fn alloc_lvis(&self, name: String, ty: Type, ssa: Option<&'ast Expr<'ast, TypedAst>>) -> &'ast VarInfo<'ast, TypedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa }))
    }

    fn alloc_expr(&self, expr: Expr<'ast, TypedAst>) -> &'ast Expr<'ast, TypedAst> {
        Box::leak(Box::new(expr))
    }
}

impl<'ast> Traverse<'ast> for TypeInfer<'ast> {
    type InAst = UntypedAst;

    type OutAst = TypedAst;

    ///
    /// Declarations
    ///

    fn trav_fundef(&mut self, fundef: Fundef<'ast, Self::InAst>) -> Fundef<'ast, Self::OutAst> {
        let Fundef { name, ret_type, args, body, decs: _ } = fundef;

        self.args = args.clone();
        self.idmap.clear();
        self.new_ids.clear();

        let new_args: Vec<&'ast Farg> = args.into_iter().map(|arg| self.trav_farg(arg)).collect();

        let mut new_body = Vec::new();
        for stmt in body {
            new_body.push(self.trav_stmt(stmt));
        }

        Fundef {
            name,
            ret_type,
            args: new_args,
            decs: self.new_ids.clone(),
            body: new_body,
        }
    }

    fn trav_farg(&mut self, arg: &'ast Farg) -> &'ast Farg {
        self.alloc_farg(arg.name.clone(), arg.ty.clone())
    }

    ///
    /// Statements
    ///

    fn trav_assign(&mut self, assign: Assign<'ast, Self::InAst>) -> Assign<'ast, Self::OutAst> {
        let (new_expr, new_ty) = self.trav_expr((*assign.expr).clone());
        let expr_ref = self.alloc_expr(new_expr);
        let new_lvis = self.alloc_lvis(assign.lvis.name.clone(), new_ty, Some(expr_ref));
        self.idmap.insert(assign.lvis as *const _, new_lvis);
        self.new_ids.push(new_lvis);
        Assign { lvis: new_lvis, expr: expr_ref }
    }

    fn trav_return(&mut self, ret: Return<'ast, Self::InAst>) -> Return<'ast, Self::OutAst> {
        let (id, _) = self.trav_id(ret.id);
        Return { id }
    }

    ///
    /// Expressions
    ///

    type ExprOut = (Expr<'ast, Self::OutAst>, Type);

    fn trav_expr(&mut self, expr: Expr<'ast, Self::InAst>) -> Self::ExprOut {
        use Expr::*;
        match expr {
            Tensor(n) => {
                let (expr, ty) = self.trav_tensor(n);
                (Tensor(expr), ty)
            },
            Binary(n) => {
                let (expr, ty) = self.trav_binary(n);
                (Binary(expr), ty)
            },
            Unary(n) => {
                let (expr, ty) = self.trav_unary(n);
                (Unary(expr), ty)
            },
            Array(n) => {
                let (expr, ty) = self.trav_array(n);
                (Array(expr), ty)
            }
            Id(n) => {
                let (id, ty) = self.trav_id(n);
                (Id(id), ty)
            },
            Bool(n) => {
                let (expr, ty) = self.trav_bool(n);
                (Bool(expr), ty)
            },
            U32(n) => {
                let (expr, ty) = self.trav_u32(n);
                (U32(expr), ty)
            },
        }
    }

    type TensorOut = (Tensor<'ast, Self::OutAst>, Type);

    fn trav_tensor(&mut self, tensor: Tensor<'ast, Self::InAst>) -> Self::TensorOut {
        let (lb, _) = self.trav_id(tensor.lb);
        let (ub, _) = self.trav_id(tensor.ub);

        let iv_new = self.alloc_lvis(tensor.iv.name.clone(), Type::scalar(BaseType::U32), None);
        self.idmap.insert(tensor.iv as *const _, iv_new);
        self.new_ids.push(iv_new);

        let mut body = Vec::new();
        for stmt in tensor.body {
            body.push(self.trav_stmt(stmt));
        }

        let (ret, _ret_ty) = self.trav_id(tensor.ret);

        let tensor = Tensor { iv: iv_new, lb, ub, ret, body };
        (tensor, Type::vector(BaseType::U32, "."))
    }

    type BinaryOut = (Binary<'ast, Self::OutAst>, Type);

    fn trav_binary(&mut self, binary: Binary<'ast, Self::InAst>) -> Self::BinaryOut {
        let (l, l_ty) = self.trav_id(binary.l);
        let (r, r_ty) = self.trav_id(binary.r);
        let ty = unifies(l_ty, r_ty).unwrap();
        (Binary { l, r, op: binary.op }, ty)
    }

    type UnaryOut = (Unary<'ast, Self::OutAst>, Type);

    fn trav_unary(&mut self, unary: Unary<'ast, Self::InAst>) -> Self::UnaryOut {
        let (r, r_ty) = self.trav_id(unary.r);
        (Unary { r, op: unary.op }, r_ty)
    }

    type ArrayOut = (Array<'ast, Self::OutAst>, Type);

    fn trav_array(&mut self, array: Array<'ast, Self::InAst>) -> Self::ArrayOut {
        let mut values = Vec::with_capacity(array.values.len());

        for value in array.values {
            let (value, _) = self.trav_id(value);
            values.push(value);
        }

        let array = Array { values };
        (array, Type::vector(BaseType::U32, "."))
    }

    ///
    /// Terminals
    ///

    type IdOut = (Id<'ast, Self::OutAst>, Type);

    fn trav_id(&mut self, id: Id<'ast, Self::InAst>) -> Self::IdOut {
        match id {
            Id::Arg(i) => {
                let ty = self.args[i].ty.clone();
                (Id::Arg(i), ty)
            },
            Id::Var(old) => {
                let new_id = self.idmap.get(&(old as *const _))
                    .expect("Id::Var referenced before its assignment was processed");
                let ty = new_id.ty.clone();
                (Id::Var(*new_id), ty)
            },
        }
    }

    type BoolOut = (bool, Type);

    fn trav_bool(&mut self, value: bool) -> Self::BoolOut {
        (value, Type::scalar(BaseType::Bool))
    }

    type U32Out = (u32, Type);

    fn trav_u32(&mut self, value: u32) -> Self::U32Out {
        (value, Type::scalar(BaseType::U32))
    }
}

fn unifies(a: Type, _b: Type) -> Result<Type, InferenceError> {
    Ok(a)
}
