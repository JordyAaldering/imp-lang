#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Const {
    Bool(bool),
    Usize(usize),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}
