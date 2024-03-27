use uuid::Uuid;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use crate::utils::logger::*;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::control_channel_receive_thread::ReceiveThread;

pub struct ReceiverTX {
    pub agent_information_packet: mpsc::UnboundedSender<BasePacket>,
    pub performance_packet: mpsc::UnboundedSender<BasePacket>,
}

pub struct ControlChannelReceiver {
    agent_id: Uuid,
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub agent_information_packet: mpsc::UnboundedReceiver<BasePacket>,
    pub performance_packet: mpsc::UnboundedReceiver<BasePacket>,
}

impl ControlChannelReceiver {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf) -> Self {
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (agent_information_packet_tx, agent_information_packet_rx) = mpsc::unbounded_channel();
        let (performance_packet_tx, performance_packet_rx) = mpsc::unbounded_channel();
        let receiver_tx = ReceiverTX {
            agent_information_packet: agent_information_packet_tx,
            performance_packet: performance_packet_tx,
        };
        let mut receive_thread = ReceiveThread::new(agent_id, socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            agent_id,
            stop_signal_tx: Some(stop_signal_tx),
            agent_information_packet: agent_information_packet_rx,
            performance_packet: performance_packet_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.agent_information_packet.close();
        self.performance_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                match stop_signal.send(()) {
                    Ok(_) => logging_info!(self.agent_id, "Control Channel: Destroyed Receiver successfully."),
                    Err(_) => logging_error!(self.agent_id, "Control Channel: Failed to destroy Receiver."),
                }
            },
            None => logging_error!(self.agent_id, "Control Channel: Failed to destroy Receiver."),
        }
    }
}
