use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct ControlPacketChannel;

impl ControlPacketChannel {
    pub fn split() -> (PacketSender, PacketReceiver) {
        let (control_reply_packet_tx, control_reply_packet_rx) = mpsc::unbounded_channel();
        let (performance_packet_tx, performance_packet_rx) = mpsc::unbounded_channel();
        (PacketSender::new(control_reply_packet_tx, performance_packet_tx),
         PacketReceiver::new(control_reply_packet_rx, performance_packet_rx))
    }
}

pub struct PacketSender {
    pub control_reply_packet: UnboundedSender<BasePacket>,
    pub performance_packet: UnboundedSender<BasePacket>,
}

impl PacketSender {
    fn new(control_reply_packet: UnboundedSender<BasePacket>, performance_packet: UnboundedSender<BasePacket>) -> Self {
        Self {
            control_reply_packet,
            performance_packet,
        }
    }
}

pub struct PacketReceiver {
    pub control_reply_packet: UnboundedReceiver<BasePacket>,
    pub performance_packet: UnboundedReceiver<BasePacket>,
}

impl PacketReceiver {
    fn new(control_reply_packet: UnboundedReceiver<BasePacket>, performance_packet: UnboundedReceiver<BasePacket>) -> Self {
        Self {
            control_reply_packet,
            performance_packet,
        }
    }
}

