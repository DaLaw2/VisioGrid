use crate::manager::utils::bounding_box::BoundingBox;

pub struct ImageResource {
    file_name: String,
    file_path: String,
    image_size: usize,
    image_id: usize,
    inference_type: usize,
    allocate: bool,
    finished: bool,
    bounding_boxes: Vec<BoundingBox>
}

impl ImageResource {
    pub fn new(image_id: usize, file_name: String, file_path: String) -> Self {
        let parts: Vec<&str> = file_name.split('_').collect();
        let inference_type = parts.get(0).unwrap().parse::<usize>().unwrap();

        Self {
            file_name,
            file_path,
            image_size: 0,
            image_id,
            inference_type,
            allocate: false,
            finished: false,
            bounding_boxes: Vec::new(),
        }
    }
}
