use tokio::sync::RwLock;
use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_CLIENT: RwLock<Client> = RwLock::new(Client::new());
}

pub struct Client {
    terminate: bool,
}

impl Client {
    pub fn new() -> Self {
        Self {
            terminate: false,
        }
    }

    pub async fn run() {

    }

    pub async fn initialize() {

    }

    pub async fn terminate() {

    }

    pub async fn send_performance() {

    }
}
