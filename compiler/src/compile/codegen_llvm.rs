use std::ffi::CString;

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
            let arg_types: Vec<LLVMTypeRef> = f.args.iter().map(|avis| {
                self.llvm_type(&avis.ty)
            }).collect();

            let fn_type = LLVMFunctionType(
                self.llvm_type(&f[f.ret.clone()].ty),
                arg_types.as_ptr() as *mut _,
                arg_types.len() as u32,
                0,
            );

            let function = LLVMAddFunction(
                self.module,
                (format!("DSL_{}", f.name)).as_ptr() as *const _,
                fn_type,
            );

            let entry = LLVMAppendBasicBlockInContext(
                self.context,
                function,
                "entry\0".as_ptr() as *const _
            );

            LLVMPositionBuilderAtEnd(self.builder, entry);

            let mut fargs = Vec::new();
            for (i, _) in f.args.iter().enumerate() {
                fargs.push(LLVMGetParam(function, i as u32));
            }

            let ret_val = match f.ret {
                ArgOrVar::Arg(i) => fargs[i],
                ArgOrVar::Var(k) => self.compile_expr(&f.ssa[k], &fargs, f),
            };

            LLVMBuildRet(self.builder, ret_val);

            function
        }
    }

    pub fn compile_expr(
        &self,
        expr: &Expr<TypedAst>,
        fargs: &Vec<LLVMValueRef>,
        fundef: &Fundef<TypedAst>,
    ) -> LLVMValueRef {
        match expr {
            Expr::Tensor(_) => {
                todo!()
            },
            Expr::Binary(Binary { l, r, op }) => {
                let l = match l {
                    ArgOrVar::Arg(i) => fargs[*i],
                    ArgOrVar::Var(k) => self.compile_expr(&fundef.ssa[*k], fargs, fundef),
                };

                let r = match r {
                    ArgOrVar::Arg(i) => fargs[*i],
                    ArgOrVar::Var(k) => self.compile_expr(&fundef.ssa[*k], fargs, fundef),
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
                let r = match r {
                    ArgOrVar::Arg(i) => fargs[*i],
                    ArgOrVar::Var(k) => self.compile_expr(&fundef.ssa[*k], fargs, fundef),
                };

                unsafe {
                    use Uop::*;
                    match op {
                        Neg => LLVMBuildNeg(self.builder, r, "negtmp\0".as_ptr() as *const _),
                        Not => LLVMBuildNot(self.builder, r, "nottmp\0".as_ptr() as *const _),
                    }
                }
            },
            Expr::U32(v) => self.build_u32(*v),
            Expr::Bool(v) => self.build_bool(*v),
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
        match t.basetype {
            BaseType::U32 => self.u32_type(),
            BaseType::Bool => self.bool_type(),
        }
    }

    pub fn u32_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt32TypeInContext(self.context) }
    }

    pub fn bool_type(&self) -> LLVMTypeRef {
        unsafe { LLVMInt1TypeInContext(self.context) }
    }
}
