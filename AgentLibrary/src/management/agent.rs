use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::{Instant, sleep};
use crate::connection::packet::Packet;
use crate::management::utils::confirm_type::ConfirmType;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::channel::ControlChannel;
use crate::connection::packet::agent_information_packet::AgentInformationPacket;
use crate::connection::packet::performance_packet::PerformancePacket;
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
    pub async fn new(socket_stream: SocketStream) -> Result<Self, String> {
        let config = Config::now().await;
        let information = serde_json::to_vec(&Manager::get_information().await)
            .map_err(|_| "Agent: Unable to serialized agent information.".to_string())?;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let mut information_confirm = false;
        control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            select! {
                biased;
                reply = control_channel_receiver.confirm_packet.recv() => {
                    return match &reply {
                        Some(packet) => {
                            match serde_json::from_slice::<ConfirmType>(packet.as_data_byte()) {
                                Ok(confirm) => {
                                    match confirm {
                                        ConfirmType::ReceiveAgentInformationSuccess => {
                                            information_confirm = true;
                                            let performance = Manager::get_performance().await;
                                            if let Ok(performance) = serde_json::to_vec(&performance) {
                                                control_channel_sender.send(PerformancePacket::new(performance)).await;
                                            }
                                            continue
                                        },
                                        ConfirmType::ReceivePerformanceSuccess => {
                                            let agent = Self {
                                                terminate: false,
                                                control_channel_sender,
                                                control_channel_receiver,
                                                data_channel_sender: None,
                                                data_channel_receiver: None,
                                            }
                                            Some(agent)
                                        },
                                    }
                                },
                                Err(_) => Err("Agent: Unable to parse confirm type.".to_string()),
                            }
                        }
                        None => Err("Agent: Channel has been closed.".to_string()),
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => {
                    if !information_confirm {
                        control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
                    } else {
                        let performance = Manager::get_performance().await;
                        if let Ok(performance) = serde_json::to_vec(&performance) {
                            control_channel_sender.send(PerformancePacket::new(performance)).await;
                        }
                    }
                    continue;
                },
            }
        }
        Err("Agent: Unable to create agent instance.".to_string())
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn send_performance(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn
}
