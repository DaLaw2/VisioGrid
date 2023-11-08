use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct ControlPacketChannel;

impl ControlPacketChannel {
    pub fn split() {
        let (sender, receiver) = mpsc::unbounded_channel();
    }
}

pub struct PacketReceiver {
    control_reply_packet: UnboundedReceiver<BasePacket>
}

impl PacketReceiver {
    fn new(control_reply_packet: UnboundedReceiver<BasePacket>) -> Self {
        Self {
            control_reply_packet,
        }
    }
}

pub struct PacketSender {
    control_reply_packet: UnboundedSender<BasePacket>
}


