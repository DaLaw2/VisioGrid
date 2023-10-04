use std::collections::BTreeSet;

pub struct IDGenerator {
    next: usize,
    available_id: BTreeSet<usize>
}

impl IDGenerator {
    pub fn new() -> Self {
        IDGenerator {
            next: 0,
            available_id: BTreeSet::new()
        }
    }

    pub fn allocate_id(&mut self) -> usize {
        if let Some(&first) = self.available_id.iter().next() {
            self.available_id.remove(&first);
            first
        } else {
            let current = self.next;
            self.next += 1;
            current
        }
    }

    pub fn free_id(&mut self, port: usize) {
        if port == self.next - 1 {
            self.next -= 1;
        } else {
            self.available_id.insert(port);
        }
    }
}
