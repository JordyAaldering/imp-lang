use std::{collections::HashMap, ffi::CString};

use llvm_sys::{core::*, prelude::*};

use crate::ast::*;

pub struct CodegenContext {
    pub context: LLVMContextRef,
    pub module: LLVMModuleRef,
    pub builder: LLVMBuilderRef,
}

impl CodegenContext {
    pub fn new(module_name: &str) -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module_name_c = CString::new(module_name).unwrap();
            let module = LLVMModuleCreateWithNameInContext(module_name_c.as_ptr(), context);
            let builder = LLVMCreateBuilderInContext(context);

            Self {
                context,
                module,
                builder,
            }
        }
    }

    pub fn compile_fundef(&self, f: &Fundef<TypedAst>) -> LLVMValueRef {
        unsafe {
            let arg_types: Vec<LLVMTypeRef> = f.args.iter().map(|key| {
                let var = &f.vars[*key];
                self.llvm_type(&var.ty)
            }).collect();

            let fn_type = LLVMFunctionType(
                self.llvm_type(&f.vars[f.ret_value].ty),
                arg_types.as_ptr() as *mut _,
                arg_types.len() as u32,
                0,
            );

            let function = LLVMAddFunction(
                self.module,
                (format!("DSL_{}", f.id)).as_ptr() as *const _,
                fn_type,
            );

            let entry = LLVMAppendBasicBlockInContext(
                self.context,
                function,
                "entry\0".as_ptr() as *const _
            );

            LLVMPositionBuilderAtEnd(self.builder, entry);

            let mut fargs = HashMap::new();
            for (i, key) in f.args.iter().enumerate() {
                let param = LLVMGetParam(function, i as u32);
                fargs.insert(f.vars[*key].id.clone(), param);
            }

            let ret_val = self.compile_expr(&f.ssa[f.ret_value], &fargs, f);

            LLVMBuildRet(self.builder, ret_val);

            function
        }
    }

    pub fn compile_expr(
        &self,
        expr: &Expr,
        fargs: &HashMap<String, LLVMValueRef>,
        fundef: &Fundef<TypedAst>,
    ) -> LLVMValueRef {
        match expr {
            Expr::U32(v) => self.build_u32(*v),
            Expr::Bool(v) => self.build_bool(*v),
            Expr::Binary(Binary { l, r, op }) => {
                let l_key = *l;
                let l_expr = &fundef.ssa.get(l_key);
                let l = if let Some(l_expr) = *l_expr {
                    self.compile_expr(l_expr, fargs, fundef)
                } else {
                    // It must be an argument
                    fargs[&fundef.vars[l_key].id]
                };

                let r_key = *r;
                let r_expr = &fundef.ssa.get(r_key);
                let r = if let Some(r_expr) = *r_expr {
                    self.compile_expr(r_expr, fargs, fundef)
                } else {
                    // It must be an argument
                    fargs[&fundef.vars[r_key].id]
                };

                unsafe {
                    use Bop::*;
                    match op {
                        Add => LLVMBuildAdd(self.builder, l, r, "addtmp\0".as_ptr() as *const _),
                        Sub => LLVMBuildSub(self.builder, l, r, "subtmp\0".as_ptr() as *const _),
                        Mul => LLVMBuildMul(self.builder, l, r, "multmp\0".as_ptr() as *const _),
                        Div => todo!(),
                        Eq => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntEQ, l, r, "eqtmp\0".as_ptr() as *const _),
                        Ne => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntNE, l, r, "netmp\0".as_ptr() as *const _),
                        Lt => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntULT, l, r, "lttmp\0".as_ptr() as *const _),
                        Le => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntULE, l, r, "letmp\0".as_ptr() as *const _),
                        Gt => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntUGT, l, r, "gttmp\0".as_ptr() as *const _),
                        Ge => LLVMBuildICmp(self.builder, llvm_sys::LLVMIntPredicate::LLVMIntUGE, l, r, "getmp\0".as_ptr() as *const _),
                    }
                }
            }
            Expr::Unary(Unary { r, op }) => {
                let r_key = *r;
                let r_expr = &fundef.ssa.get(r_key);
                let r = if let Some(r_expr) = *r_expr {
                    self.compile_expr(r_expr, fargs, fundef)
                } else {
                    // It must be an argument
                    fargs[&fundef.vars[r_key].id]
                };

                unsafe {
                    use Uop::*;
                    match op {
                        Neg => LLVMBuildNeg(self.builder, r, "negtmp\0".as_ptr() as *const _),
                        Not => LLVMBuildNot(self.builder, r, "nottmp\0".as_ptr() as *const _),
                    }
                }
            },
        }
    }

    fn build_u32(&self, v: u32) -> LLVMValueRef {
        unsafe {
            LLVMConstInt(self.u32_type(), v as u64, 0)
        }
    }

    fn build_bool(&self, v: bool) -> LLVMValueRef {
        unsafe {
            LLVMConstInt(self.u32_type(), v as u64, 0)
        }
    }

    fn llvm_type(&self, t: &Type) -> LLVMTypeRef {
        match t {
            Type::U32 => self.u32_type(),
            Type::Bool => self.bool_type(),
        }
    }

    pub fn u32_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt32TypeInContext(self.context) }
    }

    pub fn bool_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt1TypeInContext(self.context) }
    }
}
