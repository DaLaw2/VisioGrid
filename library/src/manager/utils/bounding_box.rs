use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoundingBox {
    pub name: String,
    pub x1: u32,
    pub x2: u32,
    pub y1: u32,
    pub y2: u32,
    pub confidence: f64,
}
