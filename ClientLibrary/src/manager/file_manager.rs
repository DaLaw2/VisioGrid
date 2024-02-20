use lazy_static::lazy_static;
use tokio::sync::RwLock;

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
}

pub struct FileManager;
