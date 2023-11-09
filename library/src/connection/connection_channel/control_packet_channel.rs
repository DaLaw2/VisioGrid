use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct ControlPacketChannel;

impl ControlPacketChannel {
    pub fn split() -> (PacketReceiver, PacketSender) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (PacketReceiver::new(receiver), PacketSender::new(sender))
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

impl PacketSender {
    fn new(control_reply_packet: UnboundedSender<BasePacket>) -> Self {
        Self {
            control_reply_packet,
        }
    }
}
