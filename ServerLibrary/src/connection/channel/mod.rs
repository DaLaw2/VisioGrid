pub mod control_channel_receive_thread;
pub mod control_channel_receiver;
pub mod control_channel_sender;
pub mod data_channel_receive_thread;
pub mod data_channel_receiver;
pub mod data_channel_sender;
pub mod send_thread;

use uuid::Uuid;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;

pub struct ControlChannel;

impl ControlChannel {
    pub fn new(node_id: Uuid, socket: SocketStream) -> (ControlChannelSender, ControlChannelReceiver) {
        let (socket_tx, socket_rx) = socket.into_split();
        (
            ControlChannelSender::new(node_id, socket_tx),
            ControlChannelReceiver::new(node_id, socket_rx),
        )
    }
}

pub struct DataChannel;

impl DataChannel {
    pub fn new(node_id: Uuid, socket: SocketStream) -> (DataChannelSender, DataChannelReceiver) {
        let (socket_tx, socket_rx) = socket.into_split();
        (
            DataChannelSender::new(node_id, socket_tx),
            DataChannelReceiver::new(node_id, socket_rx),
        )
    }
}
