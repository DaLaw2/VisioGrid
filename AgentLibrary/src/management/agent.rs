use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::Instant;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::channel::ControlChannel;
use crate::management::manager::Manager;
use crate::utils::config::Config;

pub struct Agent {
    terminate: bool,
    control_channel_sender: ControlChannelSender,
    control_channel_receiver: ControlChannelReceiver,
    data_channel_sender: Option<DataChannelSender>,
    data_channel_receiver: Option<DataChannelReceiver>,
}

impl Agent {
    pub async fn new(socket_stream: SocketStream) -> Self {
        let config = Config::now().await;
        let information = Manager::get_information();

        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            select! {
                biased;
                reply = control_channel_receiver.
            }
        }
        Self {}
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn send_performance(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn
}
