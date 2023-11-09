use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct DataPacketChannel;

impl DataPacketChannel {
    pub fn split() -> (PacketReceiver, PacketSender) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (PacketReceiver::new(receiver), PacketSender::new(sender))
    }
}

pub struct PacketReceiver {
    alive_reply_packet: UnboundedReceiver<BasePacket>,
}

impl PacketReceiver {
    fn new(alive_reply_packet: UnboundedReceiver<BasePacket>) -> Self {
        Self {
            alive_reply_packet
        }
    }
}

pub struct PacketSender {
    alive_reply_packet: UnboundedSender<BasePacket>,
}

impl PacketSender {
    fn new(alive_reply_packet: UnboundedSender<BasePacket>) -> Self {
        Self {
            alive_reply_packet,
        }
    }
}
