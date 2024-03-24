use std::str::FromStr;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ModelType {
    Ultralytics,
    YOLOv4,
    YOLOv7,
}

impl FromStr for ModelType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ultralytics" => Ok(ModelType::Ultralytics),
            "YOLOv4" => Ok(ModelType::YOLOv4),
            "YOLOv7" => Ok(ModelType::YOLOv7),
            _ => Err(()),
        }
    }
}

impl Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ModelType::Ultralytics => "Ultralytics",
            ModelType::YOLOv4 => "YOLOv4",
            ModelType::YOLOv7 => "YOLOv7",
        })
    }
}
