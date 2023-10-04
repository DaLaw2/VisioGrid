use std::collections::HashMap;
use crate::manager::task::image_resource::ImageResource;

pub struct Register {
    // Unwrap image_resource: inference_type => { image_id => image_resource }
    image_resource: HashMap<usize, HashMap<usize, ImageResource>>
}