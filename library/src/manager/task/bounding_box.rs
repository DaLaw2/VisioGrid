use std::fmt::{self, Formatter};

pub struct BoundingBox {
    name: String,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    confidence: f64
}

impl fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Name:{},X1:{},X2:{},Y1:{},Y2:{},Confidence:{}", self.name, self.x1, self.x2, self.y1, self.y2, self.confidence)
    }
}