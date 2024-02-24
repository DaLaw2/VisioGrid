use std::str::FromStr;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum InferenceType {
    YOLO,
    ONNX,
}

impl FromStr for InferenceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "YOLO" => Ok(InferenceType::YOLO),
            "ONNX" => Ok(InferenceType::ONNX),
            _ => Err(()),
        }
    }
}

impl Display for InferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            InferenceType::YOLO => "YOLO",
            InferenceType::ONNX => "ONNX",
        })
    }
}
