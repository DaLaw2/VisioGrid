use std::str::FromStr;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ModelType {
    TorchScript,
    PyTorch,
    ONNX,
}

impl FromStr for ModelType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TorchScript" => Ok(ModelType::TorchScript),
            "PyTorch" => Ok(ModelType::PyTorch),
            "ONNX" => Ok(ModelType::ONNX),
            _ => Err(()),
        }
    }
}

impl Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ModelType::TorchScript => "TorchScript",
            ModelType::PyTorch => "PyTorch",
            ModelType::ONNX => "ONNX",
        })
    }
}
