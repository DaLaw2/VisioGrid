use std::str::FromStr;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
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
