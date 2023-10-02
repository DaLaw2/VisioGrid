use std::collections::HashMap;
use crate::manager::task::image_resource::ImageResource;

pub struct Register {
    image_resource: HashMap<usize, HashMap<usize, ImageResource>>
}