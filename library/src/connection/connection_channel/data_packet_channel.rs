use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::connection::packet::base_packet::BasePacket;

pub struct DataPacketChannel;

impl DataPacketChannel {
    pub fn split() -> (PacketSender, PacketReceiver) {
        let (alive_reply_packet_tx, alive_reply_packet_rx) = mpsc::unbounded_channel();
        let (task_info_reply_packet_tx, task_info_reply_packet_rx) = mpsc::unbounded_channel();
        let (file_transfer_reply_packet_tx, file_transfer_reply_packet_rx) = mpsc::unbounded_channel();
        let (result_packet_tx, result_packet_rx) = mpsc::unbounded_channel();
        (PacketSender::new(alive_reply_packet_tx, task_info_reply_packet_tx, file_transfer_reply_packet_tx, result_packet_tx),
         PacketReceiver::new(alive_reply_packet_rx, task_info_reply_packet_rx, file_transfer_reply_packet_rx, result_packet_rx))
    }
}

pub struct PacketSender {
    pub alive_reply_packet: UnboundedSender<BasePacket>,
    pub task_info_reply_packet: UnboundedSender<BasePacket>,
    pub file_transfer_reply_packet: UnboundedSender<BasePacket>,
    pub result_packet: UnboundedSender<BasePacket>,
}

impl PacketSender {
    fn new(alive_reply_packet: UnboundedSender<BasePacket>, task_info_reply_packet: UnboundedSender<BasePacket>,
           file_transfer_reply_packet: UnboundedSender<BasePacket>, result_packet: UnboundedSender<BasePacket>) -> Self {
        Self {
            alive_reply_packet,
            task_info_reply_packet,
            file_transfer_reply_packet,
            result_packet,
        }
    }
}

pub struct PacketReceiver {
    pub alive_reply_packet: UnboundedReceiver<BasePacket>,
    pub task_info_reply_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_reply_packet: UnboundedReceiver<BasePacket>,
    pub result_packet: UnboundedReceiver<BasePacket>,
}

impl PacketReceiver {
    fn new(alive_reply_packet: UnboundedReceiver<BasePacket>, task_info_reply_packet: UnboundedReceiver<BasePacket>,
           file_transfer_reply_packet: UnboundedReceiver<BasePacket>, result_packet: UnboundedReceiver<BasePacket>) -> Self {
        Self {
            alive_reply_packet,
            task_info_reply_packet,
            file_transfer_reply_packet,
            result_packet,
        }
    }
}
