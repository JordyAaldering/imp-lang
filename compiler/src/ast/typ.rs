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
