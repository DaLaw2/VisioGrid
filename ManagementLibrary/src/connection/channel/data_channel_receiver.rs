use uuid::Uuid;
use tokio::sync::oneshot;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::utils::logger::*;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::data_channel_receive_thread::ReceiveThread;

pub struct ReceiverTX {
    pub alive_acknowledge_packet: UnboundedSender<BasePacket>,
    pub file_header_acknowledge_packet: UnboundedSender<BasePacket>,
    pub file_transfer_result_packet: UnboundedSender<BasePacket>,
    pub result_packet: UnboundedSender<BasePacket>,
    pub still_process_acknowledge_packet: UnboundedSender<BasePacket>,
    pub task_info_acknowledge_packet: UnboundedSender<BasePacket>,
}

pub struct DataChannelReceiver {
    agent_id: Uuid,
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub alive_acknowledge_packet: UnboundedReceiver<BasePacket>,
    pub file_header_acknowledge_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_result_packet: UnboundedReceiver<BasePacket>,
    pub result_packet: UnboundedReceiver<BasePacket>,
    pub still_process_acknowledge_packet: UnboundedReceiver<BasePacket>,
    pub task_info_acknowledge_packet: UnboundedReceiver<BasePacket>,
}

impl DataChannelReceiver {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf) -> Self {
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (alive_acknowledge_packet_tx, alive_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let (file_header_acknowledge_packet_tx, file_header_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let (file_transfer_acknowledge_packet_tx, file_transfer_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let (result_packet_tx, result_packet_rx) = mpsc::unbounded_channel();
        let (still_process_acknowledge_packet_tx, still_process_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let (task_info_acknowledge_packet_tx, task_info_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let receiver_tx = ReceiverTX {
            alive_acknowledge_packet: alive_acknowledge_packet_tx,
            file_header_acknowledge_packet: file_header_acknowledge_packet_tx,
            file_transfer_result_packet: file_transfer_acknowledge_packet_tx,
            result_packet: result_packet_tx,
            still_process_acknowledge_packet: still_process_acknowledge_packet_tx,
            task_info_acknowledge_packet: task_info_acknowledge_packet_tx,
        };
        let mut receive_thread = ReceiveThread::new(agent_id, socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            agent_id,
            stop_signal_tx: Some(stop_signal_tx),
            alive_acknowledge_packet: alive_acknowledge_packet_rx,
            file_header_acknowledge_packet: file_header_acknowledge_packet_rx,
            file_transfer_result_packet: file_transfer_acknowledge_packet_rx,
            result_packet: result_packet_rx,
            still_process_acknowledge_packet: still_process_acknowledge_packet_rx,
            task_info_acknowledge_packet: task_info_acknowledge_packet_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.alive_acknowledge_packet.close();
        self.file_header_acknowledge_packet.close();
        self.file_transfer_result_packet.close();
        self.result_packet.close();
        self.still_process_acknowledge_packet.close();
        self.task_info_acknowledge_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                match stop_signal.send(()) {
                    Ok(_) => logging_info!(self.agent_id, "Data Channel: Destroyed Receiver successfully."),
                    Err(_) => logging_error!(self.agent_id, "Data Channel: Failed to destroy Receiver."),
                }
            },
            None => logging_error!(self.agent_id, "Data Channel: Failed to destroy Receiver."),
        }
    }
}
