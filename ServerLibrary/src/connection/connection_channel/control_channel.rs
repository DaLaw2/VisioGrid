use uuid::Uuid;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::control_channel_sender::ControlChannelSender;
use crate::connection::connection_channel::control_channel_receiver::ControlChannelReceiver;

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
