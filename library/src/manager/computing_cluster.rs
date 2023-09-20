use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::logger::logger::Logger;
use std::sync::{Mutex, MutexGuard};
use crate::manager::computing_node::ComputingNode;

lazy_static! {
    static ref GLOBAL_MANAGER: Mutex<Manager> = Mutex::new(Manager::new());
}

pub struct Manager {
    nodes: HashMap<usize, ComputingNode>
}

impl Manager {
    fn new() -> Manager {
        Manager {
            nodes: HashMap::new()
        }
    }

    pub fn instance() -> MutexGuard<'static, Manager> {
        GLOBAL_MANAGER.lock().unwrap()
    }

    pub fn add_node(node: ComputingNode) {
    }

    pub fn remove_node(node_id: usize) {

    }

    // pub fn get_node(node_id: usize) -> ComputingNode {
    //
    // }
}