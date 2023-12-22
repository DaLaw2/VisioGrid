use std::str::FromStr;
use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub enum InferenceType {
    YOLO,
    PyTorch,
    TensorFlow,
    ONNX,
    Default,
}

impl FromStr for InferenceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "YOLO" => Ok(InferenceType::YOLO),
            "PyTorch" => Ok(InferenceType::PyTorch),
            "TensorFlow" => Ok(InferenceType::TensorFlow),
            "ONNX" => Ok(InferenceType::ONNX),
            _ => Ok(InferenceType::Default),
        }
    }
}

impl Display for InferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            InferenceType::YOLO => "YOLO",
            InferenceType::PyTorch => "PyTorch",
            InferenceType::TensorFlow => "TensorFlow",
            InferenceType::ONNX => "ONNX",
            InferenceType::Default => "Default",
        })
    }
}
