use uuid::Uuid;
use tokio::sync::oneshot;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::data_channel_receive_thread::ReceiveThread;

pub struct ReceiverTX {
    pub alive_reply_packet: UnboundedSender<BasePacket>,
    pub file_transfer_reply_packet: UnboundedSender<BasePacket>,
    pub result_packet: UnboundedSender<BasePacket>,
    pub still_process_reply_packet: UnboundedSender<BasePacket>,
    pub task_info_reply_packet: UnboundedSender<BasePacket>,
}

pub struct DataChannelReceiver {
    agent_id: Uuid,
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub alive_reply_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_reply_packet: UnboundedReceiver<BasePacket>,
    pub result_packet: UnboundedReceiver<BasePacket>,
    pub still_process_reply_packet: UnboundedReceiver<BasePacket>,
    pub task_info_reply_packet: UnboundedReceiver<BasePacket>,
}

impl DataChannelReceiver {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf) -> Self {
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (alive_reply_packet_tx, alive_reply_packet_rx) = mpsc::unbounded_channel();
        let (file_transfer_reply_packet_tx, file_transfer_reply_packet_rx) = mpsc::unbounded_channel();
        let (result_packet_tx, result_packet_rx) = mpsc::unbounded_channel();
        let (still_process_reply_packet_tx, still_process_reply_packet_rx) = mpsc::unbounded_channel();
        let (task_info_reply_packet_tx, task_info_reply_packet_rx) = mpsc::unbounded_channel();
        let receiver_tx = ReceiverTX {
            alive_reply_packet: alive_reply_packet_tx,
            file_transfer_reply_packet: file_transfer_reply_packet_tx,
            result_packet: result_packet_tx,
            still_process_reply_packet: still_process_reply_packet_tx,
            task_info_reply_packet: task_info_reply_packet_tx,
        };
        let mut receive_thread = ReceiveThread::new(agent_id, socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            agent_id,
            stop_signal_tx: Some(stop_signal_tx),
            alive_reply_packet: alive_reply_packet_rx,
            file_transfer_reply_packet: file_transfer_reply_packet_rx,
            result_packet: result_packet_rx,
            still_process_reply_packet: still_process_reply_packet_rx,
            task_info_reply_packet: task_info_reply_packet_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.alive_reply_packet.close();
        self.file_transfer_reply_packet.close();
        self.result_packet.close();
        self.still_process_reply_packet.close();
        self.task_info_reply_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                match stop_signal.send(()) {
                    Ok(_) => Logger::append_agent_log(self.agent_id, LogLevel::INFO, "Data Channel: Destroyed Receiver successfully.".to_string()).await,
                    Err(_) => Logger::append_agent_log(self.agent_id, LogLevel::ERROR, "Data Channel: Failed to destroy Receiver.".to_string()).await
                }
            },
            None => Logger::append_agent_log(self.agent_id, LogLevel::ERROR, "Data Channel: Failed to destroy Receiver.".to_string()).await,
        }
    }
}
