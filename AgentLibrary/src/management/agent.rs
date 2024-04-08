use std::mem;
use uuid::Uuid;
use tokio::select;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::time::Duration;
use tokio::net::TcpStream;
use std::collections::HashMap;
use tokio::time::{Instant, sleep};
use crate::utils::logger::*;
use crate::utils::config::Config;
use crate::connection::packet::Packet;
use crate::management::manager::Manager;
use crate::management::monitor::Monitor;
use crate::utils::clear_unbounded_channel;
use crate::management::utils::task_info::TaskInfo;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::file_header::FileHeader;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::channel::{ControlChannel, DataChannel};
use crate::connection::packet::performance_packet::PerformancePacket;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::packet::agent_information_packet::AgentInformationPacket;
use crate::connection::packet::alive_acknowledge_packet::AliveAcknowledgePacket;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::packet::control_acknowledge_packet::ControlAcknowledgePacket;
use crate::connection::packet::file_transfer_result_packet::FileTransferResultPacket;
use crate::connection::packet::task_info_acknowledge_packet::TaskInfoAcknowledgePacket;
use crate::connection::packet::file_header_acknowledge_packet::FileHeaderAcknowledgePacket;

pub struct Agent {
    previous_task_uuid: Option<Uuid>,
    control_channel_sender: ControlChannelSender,
    control_channel_receiver: ControlChannelReceiver,
    data_channel_sender: Option<DataChannelSender>,
    data_channel_receiver: Option<DataChannelReceiver>,
}

impl Agent {
    pub async fn new(socket_stream: SocketStream) -> Result<Self, LogEntry> {
        let config = Config::now().await;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let mut information_confirm = false;
        let information = serde_json::to_vec(&Monitor::get_system_info().await)
            .map_err(|_| error_entry!("Agent: Unable to serialized agent information."))?;
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
                packet = control_channel_receiver.agent_information_acknowledge_packet.recv() => {
                    let packet = packet
                        .ok_or(info_entry!("Agent: Channel has been closed."))?;
                    clear_unbounded_channel(&mut control_channel_receiver.agent_information_acknowledge_packet).await;
                    information_confirm = true;
                },
                packet = control_channel_receiver.performance_acknowledge_packet.recv() => {
                    let packet = packet
                        .ok_or(info_entry!("Agent: Channel has been closed."))?;
                    clear_unbounded_channel(&mut control_channel_receiver.performance_acknowledge_packet).await;
                    if !information_confirm {
                        Err(error_entry!("Agent: Agent information not acknowledge."))?;
                    }
                    let agent = Self {
                        previous_task_uuid: None,
                        control_channel_sender,
                        control_channel_receiver,
                        data_channel_sender: None,
                        data_channel_receiver: None,
                    };
                    return Ok(agent);
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        Err(info_entry!("Agent: Fail create instance. Connection Channel timeout."))
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        let for_performance = agent.clone();
        let for_management = agent;
        tokio::spawn(async move {
            Self::performance(for_performance).await;
        });
        tokio::spawn(async move {
            Self::management(for_management).await;
        });
    }

    async fn performance(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let mut polling_times = 0_u32;
        let polling_timer = Instant::now();
        let polling_interval = Duration::from_millis(config.polling_interval);
        let mut timeout_timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if Manager::get_state() == AgentState::Terminate {
                return;
            }
            if timeout_timer.elapsed() > timeout_duration {
                logging_warning!("Agent: Control Channel timeout.");
                break;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                let performance = Monitor::get_performance().await;
                if let Ok(performance_data) = serde_json::to_vec(&performance) {
                    agent.write().await.control_channel_sender.send(PerformancePacket::new(performance_data)).await;
                } else {
                    logging_error!("Agent: Unable to serialized performance data.");
                }
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.performance_acknowledge_packet.recv() => {
                    if let Some(packet) = reply {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_acknowledge_packet).await;
                        timeout_timer = Instant::now();
                    } else {
                        logging_info!("Agent: Channel has been closed.");
                        break;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        Manager::store_state(AgentState::Terminate).await;
    }

    async fn management(agent: Arc<RwLock<Agent>>) {
        loop {
            Self::refresh_state(agent.clone()).await;
            let state = Manager::get_state().await;
            match state {
                AgentState::ProcessTask => Self::process_task(agent.clone()).await,
                AgentState::Idle(idle_time) => Self::idle(agent.clone(), Duration::from_secs(idle_time)).await,
                AgentState::CreateDataChannel => Self::create_data_channel(agent.clone()).await,
                AgentState::Terminate => {
                    Self::terminate(agent.clone()).await;
                    return;
                },
                _ => {},
            }
        }
    }

    async fn refresh_state(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while Manager::get_state() != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                Manager::store_state(AgentState::Terminate).await;
                return;
            }
            let agent = agent.write().await;
            select! {
                packet = agent.control_channel_receiver.control_packet.recv() => {
                    match packet {
                        Some(packet) => {
                            clear_unbounded_channel(&mut agent.control_channel_receiver.control_packet).await;
                            match serde_json::from_slice::<AgentState>(packet.as_data_byte()) {
                                Some(state) => Manager::store_state(state).await,
                                None => {
                                    logging_error!("Agent: Unable to parse control state.");
                                    continue;
                                },
                            }
                        },
                        None => {
                            logging_warning!("Agent: Channel has been closed.");
                            Manager::store_state(AgentState::Terminate).await;
                        },
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
            agent.control_channel_sender.send(ControlAcknowledgePacket::new()).await;
            return;
        }
    }

    async fn process_task(agent: Arc<RwLock<Agent>>) {
        if let Err(entry) = Self::receive_task(agent.clone()).await {
            logging_entry!(entry);
        }
        if let Err(entry) = Self::inference_task(agent.clone()).await {
            logging_entry!(entry);
        }
    }

    async fn receive_task(agent: Arc<RwLock<Agent>>) -> Result<(), LogEntry> {
        let task_info = Self::receive_task_info(agent.clone()).await?;
        let previous_task_uuid = agent.read().await.previous_task_uuid;
        let need_receive_model = if let Some(previous_task_uuid) = previous_task_uuid {
            previous_task_uuid != task_info.uuid
        } else {
            true
        };
        if need_receive_model {
            Self::receive_file(agent.clone())?;
        }
        Self::receive_file(agent.clone())?;
        Ok(())
    }

    async fn receive_task_info(agent: Arc<RwLock<Agent>>) -> Result<TaskInfo, LogEntry> {
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        let task_info = loop {
            if Manager::get_state() == AgentState::Terminate {
                Err(info_entry!("Agent: Terminating. Receive task info cancel."))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(info_entry!("Agent: Data Channel timeout."))?;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    packet = data_channel_receiver.task_info_packet.recv() => {
                        let packet = packet
                            .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                        clear_unbounded_channel(&mut data_channel_receiver.task_info_packet).await;
                        break serde_json::from_slice::<TaskInfo>(packet.as_data_byte())
                            .map_err(|_| error_entry!("Agent: Unable to parse task info."))?;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent: Data Channel is not available."))?
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(TaskInfoAcknowledgePacket::new()).await;
        } else {
            Err(warning_entry!("Agent: Data Channel is not available."))?;
        }
        return Ok(task_info);
    }

    async fn receive_file(agent: Arc<RwLock<Agent>>, save_path: &PathBuf) -> Result<(), LogEntry> {
        let file_header = Self::receive_file_header(agent.clone()).await?;
        let file_body = Self::receive_file_body(agent.clone(), &file_header).await?;
        Self::create_file(agent, file_body, save_path).await?;
        Ok(())
    }

    async fn receive_file_header(agent: Arc<RwLock<Agent>>) -> Result<FileHeader, LogEntry> {
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        let file_header = loop {
            if Manager::get_state() == AgentState::Terminate {
                Err(info_entry!("Agent: Terminating. Receive file header cancel."))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(warning_entry!("Agent: Data Channel timeout."))?;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    packet = data_channel_receiver.task_info_packet.recv() => {
                        let packet = packet
                            .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                        clear_unbounded_channel(&mut data_channel_receiver.task_info_packet).await;
                        break serde_json::from_slice::<FileHeader>(packet.as_data_byte())
                            .map_err(|_| error_entry!("Agent: Unable to parse task info."))?;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent: Data Channel is not available."))?;
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(FileHeaderAcknowledgePacket::new()).await;
        } else {
            Err(warning_entry!("Agent: Data Channel is not available."))?;
        }
        return Ok(file_header);
    }

    async fn receive_file_body(agent: Arc<RwLock<Agent>>, file_header: &FileHeader) -> Result<Vec<Vec<u8>>, LogEntry> {
        let config = Config::now().await;
        let mut file_block: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut missing_blocks = Vec::new();
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        loop {
            if Manager::get_state() == AgentState::Terminate {
                Err(info_entry!("Agent: Terminating. Receive file header cancel."))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(warning_entry!("Agent: Data Channel timeout."))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_body_packet.recv() => {
                        let packet = &packet
                            .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                        clear_unbounded_channel(&mut data_channel_receiver.file_body_packet).await;
                        let (sequence_bytes, file_body) = packet.data.split_at(std::mem::size_of::<usize>());
                        let sequence_number = usize::from_be_bytes(sequence_bytes.try_into().map_err(|_| error_entry!("Agent: Unable to parse file body."))?);
                        file_block.insert(sequence_number, Vec::from(file_body));
                        continue;
                    },
                    packet = data_channel_receiver.file_transfer_end_packet.recv() => {
                        let packet = &packet
                            .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                        clear_unbounded_channel(&mut data_channel_receiver.file_transfer_end_packet).await;
                        for sequence_number in 0..file_header.packet_count {
                            if !file_block.contains_key(&sequence_number) {
                                missing_blocks.push(sequence_number);
                            }
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent: Data Channel is not available."))?;
            }
            if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                if missing_blocks.len() != 0_usize {
                    let result = Some(mem::take(&mut missing_blocks));
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|_| error_entry!("Agent: Unable to serialized result data."))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                } else {
                    let result: Option<Vec<usize>> = None;
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|_| error_entry!("Agent: Unable to serialized result data."))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                    let mut sorted_blocks: Vec<Vec<u8>> = Vec::with_capacity(file_header.packet_count);
                    for index in 0..file_header.packet_count {
                        if let Some(block) = file_block.remove(&index) {
                            sorted_blocks.push(block);
                        }
                    }
                    return Ok(sorted_blocks);
                };
            } else {
                Err(warning_entry!("Agent: Data Channel is not available."))?;
            }
        }
    }

    async fn create_file(agent: Arc<RwLock<Agent>>, file_body: Vec<Vec<u8>>, save_path: &PathBuf) -> Result<(), LogEntry> {

    }

    async fn inference_task(agent: Arc<RwLock<Agent>>) -> Result<(), LogEntry> {

    }

    async fn idle(agent: Arc<RwLock<Agent>>, idle_duration: Duration) {
        let config = Config::now().await;
        let timer = Instant::now();
        loop {
            if Manager::get_state() == AgentState::Terminate {
                logging_info!("Agent: Terminating. Stop idle.");
                return;
            }
            if timer.elapsed() > idle_duration {
                return;
            }
            let agent = agent.write().await;
            if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
                select! {
                    biased;
                    _ = data_channel_receiver.alive_packet.recv() => clear_unbounded_channel(&mut data_channel_receiver.alive_packet).await,
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                logging_warning!("Agent: Data Channel is not available.");
                return;
            }
            if let Some(data_channel_sender) = &mut agent.data_channel_sender {
                data_channel_sender.send(AliveAcknowledgePacket::new()).await;
            } else {
                logging_warning!("Agent: Data Channel is not available.");
                return;
            }
        }
    }

    async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let mut port: Option<u16> = None;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if Manager::get_state() == AgentState::Terminate {
                logging_info!("Agent: Terminating. Cancel create data channel.");
                return;
            }
            if timer.elapsed() > timeout_duration {
                Manager::store_state(AgentState::Terminate).await;
                logging_warning!("Agent: Control channel timout.");
                return;
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
                                logging_error!("Agent: Unable to parse port data.");
                                continue;
                            }
                        } else {
                            Manager::store_state(AgentState::Terminate).await;
                            logging_warning!("Agent: Channel has been closed.");
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
                } else {
                    logging_error!("Agent: Unable to create data channel connect.");
                    return;
                }
            } else {
                logging_error!("Agent: Port data not ready.");
                return;
            }
        }
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        logging_info!("Agent: Terminating agent.");
        let mut agent = agent.write().await;
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
}
