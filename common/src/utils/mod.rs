pub mod log_entry;
pub mod logging;
pub mod static_files;

use crate::connection::packet::base_packet::BasePacket;
pub use r#macro::*;
use tokio::sync::mpsc;

#[inline(always)]
pub async fn clear_unbounded_channel(rx: &mut mpsc::UnboundedReceiver<BasePacket>) {
    while let Ok(_) = rx.try_recv() {}
}
