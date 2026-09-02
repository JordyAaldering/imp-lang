//! # Code generation (`cg`)

mod rename_fundefs;
mod codegen_c;
mod codegen_h;
mod codegen_ffi;

pub use rename_fundefs::rename_fundefs;
pub use codegen_c::emit_c;
pub use codegen_h::emit_h;
pub use codegen_ffi::emit_ffi;
