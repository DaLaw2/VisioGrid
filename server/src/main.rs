use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use library::connection::socket::node_socket::NodeSocket;

fn main() {
    let mut sender = NodeSocket::new(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 2)), 8080), 0).unwrap();
    sender.
}