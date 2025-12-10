use std::fmt;

#[derive(Clone, Debug)]
pub struct Type {
    pub basetype: BaseType,
    pub shp: Shape,
}

#[derive(Clone, Copy, Debug)]
pub enum BaseType {
    U32,
    Bool,
}

#[derive(Clone, Debug)]
pub enum Shape {
    Scalar,
    Vector(String),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.basetype, self.shp)
    }
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BaseType::*;
        match self {
            U32 => write!(f, "u32"),
            Bool => write!(f, "bool"),
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Shape::*;
        match self {
            Scalar => Ok(()),
            Vector(n) => write!(f, "[{}]", n),
        }
    }
}
