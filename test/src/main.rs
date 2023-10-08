use std::time::Duration;
use library::connection::socket::node_socket::NodeSocket;
use library::connection::connection_channel::data_channel::DataChannel;

#[tokio::main]
async fn main() {
    let mut socket = NodeSocket::new(16384).await;
    let stream = socket.get_connection().await;
    let mut data_channel = DataChannel::new(0, stream);
    data_channel.run().await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    data_channel.disconnect().await;
}
