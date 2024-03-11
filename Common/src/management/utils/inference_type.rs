use std::str::FromStr;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum InferenceType {
    PyTorch,
    ONNX,
}

impl FromStr for InferenceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PyTorch" => Ok(InferenceType::PyTorch),
            "ONNX" => Ok(InferenceType::ONNX),
            _ => Err(()),
        }
    }
}

impl Display for InferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            InferenceType::PyTorch => "PyTorch",
            InferenceType::ONNX => "ONNX",
        })
    }
}
