use uuid::Uuid;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::data_channel_sender::DataChannelSender;
use crate::connection::connection_channel::data_channel_receiver::DataChannelReceiver;

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
