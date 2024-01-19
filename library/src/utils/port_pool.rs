use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use crate::utils::config::Config;

lazy_static! {
    static ref GLOBAL_PORT_POOL: Mutex<PortPool> = Mutex::new(PortPool::new());
}

pub struct PortPool {
    start: u16,
    end: u16,
    available: BTreeSet<u16>,
}

impl PortPool {
    fn new() -> Self {
        let [start, end] = Config::new().dedicated_port_range;
        let available = (start..end).collect::<BTreeSet<u16>>();
        Self {
            start,
            end,
            available,
        }
    }

    pub async fn allocate_port() -> Option<u16> {
        let mut port_pool = GLOBAL_PORT_POOL.lock().await;
        port_pool.available.iter().next().cloned().map(|port| {
            port_pool.available.remove(&port);
            port
        })
    }

    pub async fn free_port(port: u16) {
        let mut port_pool = GLOBAL_PORT_POOL.lock().await;
        if port >= port_pool.start && port < port_pool.end {
            port_pool.available.insert(port);
        }
    }
}
