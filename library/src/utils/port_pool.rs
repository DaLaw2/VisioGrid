use std::collections::BTreeSet;

pub struct PortPool {
    start: usize,
    end: usize,
    available: BTreeSet<usize>,
}

impl PortPool {
    pub fn new(start: usize, end: usize) -> Self {
        let available = (start..end).collect::<BTreeSet<usize>>();
        PortPool {
            start,
            end,
            available
        }
    }

    pub fn allocate_port(&mut self) -> Option<usize> {
        self.available.iter().next().cloned().map(|port| {
            self.available.remove(&port);
            port
        })
    }

    pub fn free_port(&mut self, port: usize) {
        if port >= self.start && port < self.end {
            self.available.insert(port);
        }
    }
}
