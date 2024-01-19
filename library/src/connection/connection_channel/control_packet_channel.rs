use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct ControlPacketChannel;

impl ControlPacketChannel {
    pub fn split() -> (PacketSender, PacketReceiver) {
        let (control_reply_packet_tx, control_reply_packet_rx) = mpsc::unbounded_channel();
        let (node_information_packet_tx, node_information_packet_rx) = mpsc::unbounded_channel();
        let (performance_packet_tx, performance_packet_rx) = mpsc::unbounded_channel();
        (
            PacketSender {
                control_reply_packet: control_reply_packet_tx,
                node_information_packet: node_information_packet_tx,
                performance_packet: performance_packet_tx,
            },
            PacketReceiver {
                control_reply_packet: control_reply_packet_rx,
                node_information_packet: node_information_packet_rx,
                performance_packet: performance_packet_rx,
            },
        )
    }
}

pub struct PacketSender {
    pub control_reply_packet: UnboundedSender<BasePacket>,
    pub node_information_packet: UnboundedSender<BasePacket>,
    pub performance_packet: UnboundedSender<BasePacket>,
}

pub struct PacketReceiver {
    pub control_reply_packet: UnboundedReceiver<BasePacket>,
    pub node_information_packet: UnboundedReceiver<BasePacket>,
    pub performance_packet: UnboundedReceiver<BasePacket>,
}
