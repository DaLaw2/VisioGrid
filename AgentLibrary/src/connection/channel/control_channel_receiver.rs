use tokio::sync::mpsc;
use tokio::sync::oneshot;
use crate::utils::logger::*;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::control_channel_receive_thread::ReceiveThread;

pub struct ControlChannelReceiver {
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub confirm_packet: mpsc::UnboundedReceiver<BasePacket>,
    pub data_channel_port_packet: mpsc::UnboundedReceiver<BasePacket>,
}

impl ControlChannelReceiver {
    pub fn new(socket_rx: ReadHalf) -> Self {
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (confirm_packet_tx, confirm_packet_rx) = mpsc::unbounded_channel();
        let (data_channel_port_packet_tx, data_channel_port_packet_rx) = mpsc::unbounded_channel();
        let receiver_tx = ReceiverTX {
            confirm_packet: confirm_packet_tx,
            data_channel_port_packet: data_channel_port_packet_tx,
        };
        let mut receive_thread = ReceiveThread::new(socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            stop_signal_tx: Some(stop_signal_tx),
            confirm_packet: confirm_packet_rx,
            data_channel_port_packet: data_channel_port_packet_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.confirm_packet.close();
        self.data_channel_port_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                let _ = stop_signal.send(());
                logging_info!("Control Channel: Destroyed Receiver successfully.");
            },
            None => logging_error!("Control Channel: Failed to destroy Receiver."),
        }
    }
}

pub struct ReceiverTX {
    pub confirm_packet: mpsc::UnboundedSender<BasePacket>,
    pub data_channel_port_packet: mpsc::UnboundedSender<BasePacket>,
}
