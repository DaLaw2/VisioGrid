use tokio::net::TcpStream;
use library::connection::packet::base_packet::BasePacket;
use library::connection::socket::socket_stream::SocketStream;
use library::connection::connection_channel::data_channel::DataChannel;
use library::connection::packet::definition::{PacketType, length_to_byte};

#[tokio::main]
async fn main() {
    let socket_stream = TcpStream::connect("127.0.0.1:16384").await.unwrap();
    let socket_stream = SocketStream::new(0, socket_stream);
    let mut data_channel = DataChannel::new(0, socket_stream);
    let id = PacketType::BasePacket.as_id_byte();
    let data = "Hello world.".to_string().into_bytes();
    let length = length_to_byte(data.len() + 16);
    let base_packet = BasePacket::new(length, id, data);
    data_channel.send(base_packet).await
}
