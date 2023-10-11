// Not yet complete.
pub struct Node {
    node_id: usize,
}

impl Node {
    pub fn new(node_id: usize) -> Self {
        Node {
            node_id,
        }
    }

    pub fn get_node_id(&self) -> usize {
        self.node_id
    }
}
