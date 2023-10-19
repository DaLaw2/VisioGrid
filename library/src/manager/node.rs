use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::socket::socket_stream::SocketStream;

pub struct Node {
    node_id: usize,
    control_channel: ControlChannel,
    data_channel: Option<DataChannel>,
}

impl Node {
    pub fn new(node_id: usize, socket_stream: SocketStream) -> Self {
        Node {
            node_id,
            control_channel: ControlChannel::new(node_id, socket_stream),
            data_channel: None,
        }
    }



    pub fn get_id(&self) -> usize {
        self.node_id
    }
}
