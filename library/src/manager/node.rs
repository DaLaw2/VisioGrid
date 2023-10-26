use tokio::time::sleep;
use std::time::Duration;
use tokio::net::TcpListener;
use std::collections::VecDeque;
use crate::utils::port_pool::PortPool;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::definition::PerformanceData;
use crate::connection::socket::socket_stream::SocketStream;
use crate::manager::utils::infeerence_resource::InferenceResource;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;

pub struct Node {
    node_id: usize,
    control_channel: ControlChannel,
    data_channel: Option<DataChannel>,
    process_queue: VecDeque<InferenceResource>,
    performance_data: PerformanceData,
}

impl Node {
    pub fn new(node_id: usize, socket_stream: SocketStream) -> Self {
        Node {
            node_id,
            control_channel: ControlChannel::new(node_id, socket_stream),
            data_channel: None,
            process_queue: VecDeque::new(),
            performance_data: PerformanceData::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub async fn run() {

    }

    pub fn get_id(&self) -> usize {
        self.node_id
    }

    async fn create_data_channel(&mut self) {
        let (listener, port) = loop {
            let port = match PortPool::instance().await.allocate_port() {
                Some(port) => port,
                None => continue,
            };
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(listener) => break (listener, port),
                Err(_) => continue,
            }
        };
        self.control_channel.send(DataChannelPortPacket::new(port)).await;
        let (stream, address) = loop {
            match listener.accept().await {
                Ok(connection) => break connection,
                Err(_) => {
                    self.control_channel.send(DataChannelPortPacket::new(port)).await;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        };
        let socket_stream =  SocketStream::new(stream, address);
        self.data_channel = Some(DataChannel::new(self.node_id, socket_stream));
        Logger::instance().await.append_node_log(self.node_id, LogLevel::INFO, format!("Node {} success create data channel.", self.node_id));
    }

    async fn performance() {

    }
}
