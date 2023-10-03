use crate::manager::task::bounding_box::BoundingBox;

pub struct ImageResource {
    source_ip: String,
    file_name: String,
    file_path: String,
    image_size: usize,
    image_id: usize,
    inference_type: usize,
    allocate: bool,
    finished: bool,
    fail_times: usize,
    cost_time: f64,
    bounding_boxes: Vec<BoundingBox>
}

impl ImageResource {
    pub fn new(image_id: usize, file_name: String, file_path: String) -> Self {

        Self {
            source_ip: "".to_string(),
            file_name,
            file_path,
            image_size: 0,
            image_id,
            inference_type: 0,
            allocate: false,
            finished: false,
            fail_times: 0,
            cost_time: 0.0,
            bounding_boxes: vec![],
        }
    }
}