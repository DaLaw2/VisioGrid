use tokio::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::{Instant, sleep};
use Common::{error_entry, info_entry};
use Common::management::utils::performance::Performance;
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
use crate::connection::channel::{ControlChannel, DataChannel};
use crate::connection::packet::agent_information_packet::AgentInformationPacket;
use crate::connection::packet::alive_reply_packet::AliveReplyPacket;
use crate::connection::packet::performance_packet::PerformancePacket;
use crate::{logging_error, logging_info, logging_warning};
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
            .map_err(|_| error_entry!("Agent: Unable to serialized agent information."))?;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let mut information_confirm = false;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            if timer.elapsed() > polling_times * polling_interval {
                if !information_confirm {
                    control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
                } else {
                    let performance = serde_json::to_vec(&Monitor::get_performance().await)
                        .map_err(|_| error_entry!("Agent: Unable to serialized performance data."))?;
                    control_channel_sender.send(PerformancePacket::new(performance)).await;
                }
                polling_times += 1;
            }
            select! {
                biased;
                reply = control_channel_receiver.confirm_packet.recv() => {
                    let packet = reply
                        .ok_or(info_entry!("Agent: Channel has been closed."))?;
                    clear_unbounded_channel(&mut control_channel_receiver.confirm_packet).await;
                    let confirm = serde_json::from_slice::<ConfirmType>(packet.as_data_byte())
                        .map_err(|_| error_entry!("Agent: Unable to parse confirm type."))?;
                    match confirm {
                        ConfirmType::ReceiveAgentInformationSuccess => {
                            information_confirm = true;
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
        Err(info_entry!("Agent: Fail create instance. Connection Channel timeout."))
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        let for_performance = agent.clone();
        let for_management = agent.clone();
        tokio::spawn(async move {
            Self::performance(for_performance).await;
        });
        tokio::spawn(async move {
            Self::create_data_channel(agent).await;
        });
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        logging_info!("Agent: Terminating agent.");
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
        logging_info!("Agent: Termination complete.");
    }

    async fn performance(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let mut polling_times = 0_u32;
        let polling_timer = Instant::now();
        let polling_interval = Duration::from_millis(config.polling_interval);
        let mut timeout_timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while !agent.read().await.terminate {
            if timeout_timer.elapsed() > timeout_duration {
                logging_warning!("Agent: Control Channel timeout.");
                Agent::terminate(agent).await;
                return;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                let performance = Monitor::get_performance().await;
                if let Ok(performance) = serde_json::to_vec(&performance) {
                    agent.write().await.control_channel_sender.send(PerformancePacket::new(performance)).await;
                } else {
                    logging_error!("Agent: Unable to serialized performance data.");
                }
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.confirm_packet.recv() => {
                    if let Some(packet) = reply {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.confirm_packet).await;
                        if let Ok(_) = serde_json::from_slice::<ConfirmType>(packet.as_data_byte()) {
                            timeout_timer = Instant::now();
                        } else {
                            logging_error!("Agent: Unable to parse confirm data.");
                        }
                    } else {
                        logging_info!("Agent: Channel has been closed.");
                        return;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
    }

    async fn process_task(agent: Arc<RwLock<Agent>>) {

    }

    async fn idle(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        while !agent.read().await.terminate {
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    biased;
                    reply = data_channel_receiver.alive_packet.recv() => {},
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
            if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
                data_channel_sender.send(AliveReplyPacket::new()).await;
            }
        }
    }

    async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let mut port: Option<u16> = None;
        while !agent.read().await.terminate {
            if agent.read().await.data_channel_sender.is_some() {
                sleep(Duration::from_millis(config.internal_timestamp)).await;
            }
            {
                let mut agent = agent.write().await;
                select! {
                    biased;
                    reply = agent.control_channel_receiver.data_channel_port_packet.recv() => {
                        if let Some(packet) = reply {
                            clear_unbounded_channel(&mut agent.control_channel_receiver.data_channel_port_packet).await;
                            let bytes = packet.as_data_byte();
                            if bytes.len() == 2 {
                                port = Some(u16::from_be_bytes([bytes[0], bytes[1]]))
                            } else {
                                logging_info!("Agent: Unable to parse port data.");
                                continue;
                            }
                        } else {
                            logging_info!("Agent: Channel has been closed.");
                            return;
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
            if let Some(port) = port {
                let full_address = format!("{}:{}", config.management_address, port);
                if let Ok(tcp_stream) = TcpStream::connect(&full_address).await {
                    let socket_stream = SocketStream::new(tcp_stream);
                    let (data_channel_sender, data_channel_receiver) = DataChannel::new(socket_stream);
                    let mut agent = agent.write().await;
                    agent.data_channel_sender = Some(data_channel_sender);
                    agent.data_channel_receiver = Some(data_channel_receiver);
                }
            } else {
                logging_info!("Agent: Internal server error.");
                return;
            }
        }
    }
}
