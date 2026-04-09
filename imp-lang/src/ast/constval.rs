#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Const {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    Usize(usize),
    F32(f32),
    F64(f64),
}
