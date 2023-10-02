use std::collections::{HashSet, VecDeque};

pub struct PortPool {
    queue: VecDeque<usize>,
    set: HashSet<usize>
}

impl PortPool {
    pub fn new(start: usize, end: usize) -> Self {
        let mut queue: VecDeque<usize> = VecDeque::new();
        let mut set: HashSet<usize> = HashSet::new();
        for port in start..end {
            queue.push_back(port);
            set.insert(port);
        }
        Self {
            queue,
            set
        }
    }

    pub fn allocate_port(&mut self) -> Option<usize> {
        if let Some(port) = self.queue.pop_front() {
            self.set.remove(&port);
            Some(port)
        } else {
            None
        }
    }

    pub fn free_port(&mut self, port: usize) {
        if self.set.insert(port) {
            self.queue.push_back(port)
        }
    }
}