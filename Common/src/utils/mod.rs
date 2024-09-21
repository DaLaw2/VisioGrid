pub mod log_entry;
pub mod logging;
pub mod static_files;

use crate::connection::packet::base_packet::BasePacket;
use tokio::sync::mpsc;
pub use Macro::*;

#[inline(always)]
pub async fn clear_unbounded_channel(rx: &mut mpsc::UnboundedReceiver<BasePacket>) {
    while let Ok(_) = rx.try_recv() {}
}
