use std::fmt;

#[derive(Clone, Debug)]
pub struct MaybeType(pub Option<Type>);

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

impl MaybeType {
    pub fn unwrap(self) -> Type {
        self.0.expect("expected a type")
    }
}

impl Type {
    pub fn scalar(basetype: BaseType) -> Self {
        Self { basetype, shp: Shape::Scalar }
    }

    pub fn vector(basetype: BaseType, extent: &str) -> Self {
        Self { basetype, shp: Shape::Vector(extent.to_owned()) }
    }
}

impl fmt::Display for MaybeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(ty) => write!(f, "{}", ty),
            None => write!(f, "⊥"),
        }
    }
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
