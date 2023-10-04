use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use library::connection::packet::definition::{length_to_byte, PacketType};

#[tokio::main]
async fn main() {
    let mut socket_stream = TcpStream::connect("127.0.0.1:16384").await.unwrap();
    let packet_type = PacketType::BasePacket.as_id_byte();
    let data = "Hello world.".to_string().into_bytes();
    let length = length_to_byte(data.len() + 16);
    loop {
        socket_stream.write_all(&length).await.expect("Fail send packet.");
        socket_stream.write_all(&packet_type).await.expect("Fail send packet.");
        socket_stream.write_all(&data).await.expect("Fail send packet.");
        socket_stream.flush().await.expect("Fail send packet");
    }
}