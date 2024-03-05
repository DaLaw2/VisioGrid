use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::{Instant, sleep};
use crate::utils::clear_unbounded_channel;
use crate::utils::logger::{Logger, LogLevel};
use crate::utils::logger::LogEntry;
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
use crate::management::monitor::Monitor;
use crate::utils::config::Config;

pub struct Agent {
    terminate: bool,
    control_channel_sender: ControlChannelSender,
    control_channel_receiver: ControlChannelReceiver,
    data_channel_sender: Option<DataChannelSender>,
    data_channel_receiver: Option<DataChannelReceiver>,
}

impl Agent {
    pub async fn new(socket_stream: SocketStream) -> Result<Self, LogEntry> {
        let config = Config::now().await;
        let information = serde_json::to_vec(&Monitor::get_system_info().await)
            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized agent information.".to_string()))?;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let mut information_confirm = false;
        control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            if timer.elapsed() > polling_interval * polling_times {
                if !information_confirm {
                    control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
                } else {
                    let performance = Monitor::get_performance().await;
                    let performance = serde_json::to_vec(&performance)
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized performance data.".to_string()))?;
                    control_channel_sender.send(PerformancePacket::new(performance)).await;
                }
                polling_times += 1;
            }
            select! {
                biased;
                reply = control_channel_receiver.confirm_packet.recv() => {
                    let packet = reply
                        .ok_or(LogEntry::new(LogLevel::INFO, "Agent: Channel has been closed.".to_string()))?;
                    clear_unbounded_channel(&mut control_channel_receiver.confirm_packet).await;
                    let confirm = serde_json::from_slice::<ConfirmType>(packet.as_data_byte())
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to parse confirm type.".to_string()))?;
                    match confirm {
                        ConfirmType::ReceiveAgentInformationSuccess => {
                            information_confirm = true;
                            let performance = Monitor::get_performance().await;
                            let performance = serde_json::to_vec(&performance)
                                .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized performance data.".to_string()))?;
                            control_channel_sender.send(PerformancePacket::new(performance)).await;
                            continue
                        },
                        ConfirmType::ReceivePerformanceSuccess => {
                            let agent = Self {
                                terminate: false,
                                control_channel_sender,
                                control_channel_receiver,
                                data_channel_sender: None,
                                data_channel_receiver: None,
                            };
                            return Ok(agent)
                        },
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        Err(LogEntry::new(LogLevel::INFO, "Agent: Fail create instance. Connection Channel timeout.".to_string()))
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        let for_performance = agent.clone();
        let for_management = agent.clone();
        tokio::spawn(async move {
            Self::performance(for_performance).await;
        });
        tokio::spawn(async move {
            Self::task_management(for_management).await;
        });
        tokio::spawn(async move {
            Self::create_data_channel(agent).await;
        });
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        Logger::add_system_log(LogLevel::INFO, "Agent: Terminating agent.".to_string()).await;
        let mut agent = agent.write().await;
        agent.terminate = true;
        agent.control_channel_sender.disconnect().await;
        agent.control_channel_receiver.disconnect().await;
        if let Some(data_channel_sender) = &mut agent.data_channel_sender {
            data_channel_sender.disconnect().await;
        }
        if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
            data_channel_receiver.disconnect().await;
        }
        Logger::add_system_log(LogLevel::INFO, "Agent: Termination complete.".to_string()).await;
    }

    pub async fn performance(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn task_management(agent: Arc<RwLock<Agent>>) {

    }

    pub async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        loop {
            let port = loop {
                let control_channel_receiver = &mut agent.write().await.control_channel_receiver;
                select! {
                    reply = control_channel_receiver.data_channel_port_packet.recv() => {
                        match &reply {
                            Some(packet) => {
                                match packet.as_data_byte().try_into() {
                                    Ok(bytes) => {
                                        let bytes: [u8; 2] = bytes;
                                        break u16::from_be_bytes(bytes);
                                    },
                                    _ => Logger::add_system_log(LogLevel::ERROR, "Agent: Unable to parse port data.".to_string()).await,
                                }
                            },
                            None => Logger::add_system_log(LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await,
                        };
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            };

        }
    }
}
