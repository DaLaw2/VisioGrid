use std::collections::BTreeSet;

pub struct PortPool {
    start: usize,
    end: usize,
    available_port: BTreeSet<usize>,
}

impl PortPool {
    pub fn new(start: usize, end: usize) -> Self {
        let available_port = (start..end).collect::<BTreeSet<usize>>();
        PortPool {
            start,
            end,
            available_port
        }
    }

    pub fn allocate_port(&mut self) -> Option<usize> {
        self.available_port.iter().next().cloned().map(|port| {
            self.available_port.remove(&port);
            port
        })
    }

    pub fn free_port(&mut self, port: usize) {
        if port >= self.start && port < self.end {
            self.available_ports.insert(port);
        }
    }
}