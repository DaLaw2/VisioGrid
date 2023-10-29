use tokio::time::sleep;
use std::time::Duration;
use tokio::net::TcpListener;
use std::collections::VecDeque;
use crate::utils::port_pool::PortPool;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::utils::performance::Performance;
use crate::manager::utils::image_resource::ImageResource;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;

pub struct Node {
    id: usize,
    control_channel: ControlChannel,
    data_channel: Option<DataChannel>,
    process_queue: VecDeque<ImageResource>,
    pub idle_performance: Performance,
    pub realtime_usage: Performance,
}

impl Node {
    pub fn new(id: usize, socket_stream: SocketStream) -> Self {
        Self {
            id,
            control_channel: ControlChannel::new(id, socket_stream),
            data_channel: None,
            process_queue: VecDeque::new(),
            idle_performance: Performance::new(0.0, 0.0, 0.0, 0.0),
            realtime_usage: Performance::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub async fn run() {

    }

    pub fn get_id(&self) -> usize {
        self.id
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
        self.data_channel = Some(DataChannel::new(self.id, socket_stream));
        Logger::instance().await.append_node_log(self.id, LogLevel::INFO, format!("Node {} success create data channel.", self.id));
    }

    async fn performance(&mut self) {

    }
}
