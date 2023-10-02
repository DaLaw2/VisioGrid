use std::collections::BTreeSet;

pub struct IDGenerator {
    available: BTreeSet<usize>,
    next: usize,
}

impl IDGenerator {
    pub fn new() -> Self {
        IDGenerator {
            available: BTreeSet::new(),
            next: 0,
        }
    }

    pub fn allocate_id(&mut self) -> usize {
        if let Some(&first) = self.available.iter().next() {
            self.available.remove(&first);
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
            self.available.insert(port);
        }
    }
}
