use tokio::sync::oneshot;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::utils::logging::*;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::data_channel_receive_thread::ReceiveThread;

pub struct ReceiverTX {
    pub alive_packet: UnboundedSender<BasePacket>,
    pub file_body_packet: UnboundedSender<BasePacket>,
    pub file_header_packet: UnboundedSender<BasePacket>,
    pub file_transfer_end_packet: UnboundedSender<BasePacket>,
    pub result_acknowledge_packet: UnboundedSender<BasePacket>,
    pub still_process_packet: UnboundedSender<BasePacket>,
    pub task_info_packet: UnboundedSender<BasePacket>,
}

pub struct DataChannelReceiver {
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub alive_packet: UnboundedReceiver<BasePacket>,
    pub file_body_packet: UnboundedReceiver<BasePacket>,
    pub file_header_packet: UnboundedReceiver<BasePacket>,
    pub file_transfer_end_packet: UnboundedReceiver<BasePacket>,
    pub result_acknowledge_packet: UnboundedReceiver<BasePacket>,
    pub still_process_packet: UnboundedReceiver<BasePacket>,
    pub task_info_packet: UnboundedReceiver<BasePacket>,
}

impl DataChannelReceiver {
    pub fn new(socket_rx: ReadHalf) -> Self {
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (alive_packet_tx, alive_packet_rx) = mpsc::unbounded_channel();
        let (file_body_packet_tx, file_body_packet_rx) = mpsc::unbounded_channel();
        let (file_header_packet_tx, file_header_packet_rx) = mpsc::unbounded_channel();
        let (file_transfer_end_packet_tx, file_transfer_end_packet_rx) = mpsc::unbounded_channel();
        let (result_acknowledge_packet_tx, result_acknowledge_packet_rx) = mpsc::unbounded_channel();
        let (still_process_packet_tx, still_process_packet_rx) = mpsc::unbounded_channel();
        let (task_info_packet_tx, task_info_packet_rx) = mpsc::unbounded_channel();
        let receiver_tx = ReceiverTX {
            alive_packet: alive_packet_tx,
            file_body_packet: file_body_packet_tx,
            file_header_packet: file_header_packet_tx,
            file_transfer_end_packet: file_transfer_end_packet_tx,
            result_acknowledge_packet: result_acknowledge_packet_tx,
            still_process_packet: still_process_packet_tx,
            task_info_packet: task_info_packet_tx,
        };
        let mut receive_thread = ReceiveThread::new(socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            stop_signal_tx: Some(stop_signal_tx),
            alive_packet: alive_packet_rx,
            file_body_packet: file_body_packet_rx,
            file_header_packet: file_header_packet_rx,
            file_transfer_end_packet: file_transfer_end_packet_rx,
            result_acknowledge_packet: result_acknowledge_packet_rx,
            still_process_packet: still_process_packet_rx,
            task_info_packet: task_info_packet_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.alive_packet.close();
        self.file_body_packet.close();
        self.file_header_packet.close();
        self.file_transfer_end_packet.close();
        self.still_process_packet.close();
        self.task_info_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                if stop_signal.send(()).is_ok() {
                    logging_information!("Data Channel", "Successfully destroyed the Receiver", "");
                } else {
                    logging_error!("Data Channel", "Failed to destroy Receiver", "");
                }
            },
            None => logging_error!("Data Channel", "Failed to destroy Receiver", ""),
        }
    }
}
