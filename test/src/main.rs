use library::connection::connection_channel::data_channel::DataChannel;
use library::connection::socket::node_socket::NodeSocket;

#[tokio::main]
async fn main() {
    let mut socket = NodeSocket::new(16384).await;
    let stream = socket.get_connection().await;
    let mut data_channel = DataChannel::new(0, stream);
    loop {
        data_channel.run().await;
    }
}