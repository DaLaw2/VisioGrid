use crate::connection::channel::control_channel_receive_thread::ReceiveThread;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::utils::create_unbounded_channels;
use crate::utils::logging::*;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub struct ReceiverTX {
    pub agent_info_ack_packet: mpsc::UnboundedSender<BasePacket>,
    pub control_packet: mpsc::UnboundedSender<BasePacket>,
    pub data_channel_port_packet: mpsc::UnboundedSender<BasePacket>,
    pub performance_ack_packet: mpsc::UnboundedSender<BasePacket>,
}

pub struct ControlChannelReceiver {
    stop_signal_tx: Option<oneshot::Sender<()>>,
    pub agent_info_ack_packet: mpsc::UnboundedReceiver<BasePacket>,
    pub control_packet: mpsc::UnboundedReceiver<BasePacket>,
    pub data_channel_port_packet: mpsc::UnboundedReceiver<BasePacket>,
    pub performance_ack_packet: mpsc::UnboundedReceiver<BasePacket>,
}

impl ControlChannelReceiver {
    pub fn new(socket_rx: ReadHalf) -> Self {
        create_unbounded_channels!(4);
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let receiver_tx = ReceiverTX {
            agent_info_ack_packet: channel_0_tx,
            control_packet: channel_1_tx,
            data_channel_port_packet: channel_2_tx,
            performance_ack_packet: channel_3_tx,
        };
        let mut receive_thread = ReceiveThread::new(socket_rx, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            stop_signal_tx: Some(stop_signal_tx),
            agent_info_ack_packet: channel_0_rx,
            control_packet: channel_1_rx,
            data_channel_port_packet: channel_2_rx,
            performance_ack_packet: channel_3_rx,
        }
    }

    pub async fn disconnect(&mut self) {
        self.agent_info_ack_packet.close();
        self.control_packet.close();
        self.data_channel_port_packet.close();
        self.performance_ack_packet.close();
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                if stop_signal.send(()).is_err() {
                    logging_error!(NetworkEntry::DestroyInstanceError);
                }
            }
            None => logging_error!(NetworkEntry::DestroyInstanceError),
        }
    }
}
