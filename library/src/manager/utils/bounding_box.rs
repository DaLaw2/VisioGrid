use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::fmt::{self, Display, Formatter};

#[derive(PartialEq)]
pub struct BoundingBox {
    name: String,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    confidence: f64
}

impl Display for BoundingBox {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Name:{},X1:{},X2:{},Y1:{},Y2:{},Confidence:{}", self.name, self.x1, self.x2, self.y1, self.y2, self.confidence)
    }
}

impl Hash for BoundingBox {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.x1.to_bits().hash(state);
        self.x2.to_bits().hash(state);
        self.y1.to_bits().hash(state);
        self.y2.to_bits().hash(state);
        self.confidence.to_bits().hash(state);
    }
}

impl Eq for BoundingBox {}

impl PartialOrd for BoundingBox {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BoundingBox {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
            .then_with(|| self.x1.partial_cmp(&other.x1).unwrap_or(Ordering::Equal))
            .then_with(|| self.x2.partial_cmp(&other.x2).unwrap_or(Ordering::Equal))
            .then_with(|| self.y1.partial_cmp(&other.y1).unwrap_or(Ordering::Equal))
            .then_with(|| self.y2.partial_cmp(&other.y2).unwrap_or(Ordering::Equal))
            .then_with(|| self.confidence.partial_cmp(&other.confidence).unwrap_or(Ordering::Equal))
    }
}