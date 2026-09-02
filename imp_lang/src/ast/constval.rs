#[derive(Copy, Clone, Debug, PartialEq)]
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

impl ToString for Const {
    fn to_string(&self) -> String {
        match self {
            Self::Bool(v) => v.to_string(),
            Self::Usize(v) => v.to_string(),
            Self::U32(v) => v.to_string(),
            Self::U64(v) => v.to_string(),
            Self::I32(v) => v.to_string(),
            Self::I64(v) => v.to_string(),
            Self::F32(v) => v.to_string(),
            Self::F64(v) => v.to_string(),
        }
    }
}
