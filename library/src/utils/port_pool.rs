use lazy_static::lazy_static;
use std::collections::BTreeSet;
use tokio::sync::{Mutex, MutexGuard};
use crate::utils::config::Config;

lazy_static! {
    static ref GLOBAL_PORT_POOL: Mutex<PortPool> = Mutex::new(PortPool::new());
}

pub struct PortPool {
    start: usize,
    end: usize,
    available: BTreeSet<usize>,
}

impl PortPool {
    fn new() -> Self {
        //沒有更好的方法了嗎？
        let (start, end) = Config::new().dedicated_port_range;
        let available = (start..end).collect::<BTreeSet<usize>>();
        Self {
            start,
            end,
            available
        }
    }

    pub async fn instance() -> MutexGuard<'static, PortPool> {
        GLOBAL_PORT_POOL.lock().await
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
