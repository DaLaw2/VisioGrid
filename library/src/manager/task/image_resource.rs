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

}