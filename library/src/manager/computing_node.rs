pub struct ComputingNode {
    node_id: usize,
}

impl ComputingNode {
    pub fn new(node_id: usize) -> ComputingNode {
        ComputingNode {
            node_id,
        }
    }

    pub fn get_node_id(&self) -> usize {
        self.node_id
    }
}