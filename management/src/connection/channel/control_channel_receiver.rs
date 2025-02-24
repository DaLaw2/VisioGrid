use crate::connection::channel::control_channel_receive_thread::ReceiveThread;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::utils::create_unbounded_channels;
use crate::utils::logging::*;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use uuid::Uuid;

pub struct ReceiverTX {
    pub agent_info_packet: UnboundedSender<BasePacket>,
    pub control_ack_packet: UnboundedSender<BasePacket>,
    pub performance_packet: UnboundedSender<BasePacket>,
}

pub struct ControlChannelReceiver {
    agent_id: Uuid,
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub agent_info_packet: UnboundedReceiver<BasePacket>,
    pub control_ack_packet: UnboundedReceiver<BasePacket>,
    pub performance_packet: UnboundedReceiver<BasePacket>,
}

impl ControlChannelReceiver {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf) -> Self {
        create_unbounded_channels!(3);
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let receiver_tx = ReceiverTX {
            agent_info_packet: channel_0_tx,
            control_ack_packet: channel_1_tx,
            performance_packet: channel_2_tx,
        };
        let mut receive_thread = ReceiveThread::new(agent_id, socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            agent_id,
            stop_signal_tx: Some(stop_signal_tx),
            agent_info_packet: channel_0_rx,
            control_ack_packet: channel_1_rx,
            performance_packet: channel_2_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.agent_info_packet.close();
        self.control_ack_packet.close();
        self.performance_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                if stop_signal.send(()).is_err() {
                    logging_error!(self.agent_id, NetworkEntry::DestroyInstanceError, "");
                }
            }
            None => logging_error!(self.agent_id, NetworkEntry::DestroyInstanceError, "")
        }
    }
}
