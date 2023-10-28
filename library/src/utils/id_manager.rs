use std::collections::BTreeSet;

pub struct IDManager {
    next: usize,
    available: BTreeSet<usize>
}

impl IDManager {
    pub fn new() -> Self {
        Self {
            next: 0,
            available: BTreeSet::new()
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
