use crate::connection::channel::data_channel_receive_thread::ReceiveThread;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::utils::create_unbounded_channels;
use crate::utils::logging::*;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use uuid::Uuid;

pub struct ReceiverTX {
    pub alive_ack_packet: UnboundedSender<BasePacket>,
    pub file_body_packet: UnboundedSender<BasePacket>,
    pub file_header_ack_packet: UnboundedSender<BasePacket>,
    pub file_header_packet: UnboundedSender<BasePacket>,
    pub file_transfer_end_packet: UnboundedSender<BasePacket>,
    pub file_transfer_result_packet: UnboundedSender<BasePacket>,
    pub still_process_ack_packet: UnboundedSender<BasePacket>,
    pub task_info_ack_packet: UnboundedSender<BasePacket>,
    pub task_result_packet: UnboundedSender<BasePacket>,
}

pub struct DataChannelReceiver {
    agent_id: Uuid,
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub alive_ack_packet: UnboundedReceiver<BasePacket>,
    pub file_body_packet: UnboundedReceiver<BasePacket>,
    pub file_header_ack_packet: UnboundedReceiver<BasePacket>,
    pub file_header_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_end_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_result_packet: UnboundedReceiver<BasePacket>,
    pub still_process_ack_packet: UnboundedReceiver<BasePacket>,
    pub task_info_ack_packet: UnboundedReceiver<BasePacket>,
    pub task_result_packet: UnboundedReceiver<BasePacket>,
}

impl DataChannelReceiver {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf) -> Self {
        create_unbounded_channels!(9);
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let receiver_tx = ReceiverTX {
            alive_ack_packet: channel_0_tx,
            file_body_packet: channel_1_tx,
            file_header_ack_packet: channel_2_tx,
            file_header_packet: channel_3_tx,
            file_transfer_end_packet: channel_4_tx,
            file_transfer_result_packet: channel_5_tx,
            still_process_ack_packet: channel_6_tx,
            task_info_ack_packet: channel_7_tx,
            task_result_packet: channel_8_tx,
        };
        let mut receive_thread = ReceiveThread::new(agent_id, socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            agent_id,
            stop_signal_tx: Some(stop_signal_tx),
            alive_ack_packet: channel_0_rx,
            file_body_packet: channel_1_rx,
            file_header_ack_packet: channel_2_rx,
            file_header_packet: channel_3_rx,
            file_transfer_end_packet: channel_4_rx,
            file_transfer_result_packet: channel_5_rx,
            still_process_ack_packet: channel_6_rx,
            task_info_ack_packet: channel_7_rx,
            task_result_packet: channel_8_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.alive_ack_packet.close();
        self.file_body_packet.close();
        self.file_header_ack_packet.close();
        self.file_header_packet.close();
        self.file_transfer_end_packet.close();
        self.file_transfer_result_packet.close();
        self.still_process_ack_packet.close();
        self.task_info_ack_packet.close();
        self.task_result_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                if stop_signal.send(()).is_err() {
                    logging_error!(self.agent_id, NetworkEntry::DestroyInstanceError, "");
                }
            }
            None => logging_error!(self.agent_id, NetworkEntry::DestroyInstanceError, ""),
        }
    }
}
