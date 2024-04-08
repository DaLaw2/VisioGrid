pub mod logging;
pub mod static_files;

use tokio::sync::mpsc;
use crate::connection::packet::base_packet::BasePacket;

#[inline(always)]
pub async fn clear_unbounded_channel(rx: &mut mpsc::UnboundedReceiver<BasePacket>) {
    while let Ok(_) = rx.try_recv() {}
}
