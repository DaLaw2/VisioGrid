use std::mem;
use uuid::Uuid;
use tokio::select;
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::RwLock;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::time::{Instant, sleep};
use crate::utils::logging::*;
use crate::utils::config::Config;
use crate::connection::packet::Packet;
use crate::management::monitor::Monitor;
use crate::utils::clear_unbounded_channel;
use crate::management::utils::task_info::TaskInfo;
use crate::management::utils::model_type::ModelType;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::task_result::TaskResult;
use crate::management::utils::bounding_box::BoundingBox;
use crate::connection::packet::result_packet::ResultPacket;
use crate::management::calculate_manager::CalculateManager;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::channel::{ControlChannel, DataChannel};
use crate::connection::packet::performance_packet::PerformancePacket;
use crate::management::utils::file_transfer_result::FileTransferResult;
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
    pub state: AgentState,
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
            .map_err(|err| error_entry!("Agent", "Unable to serialize data", format!("Err: {err}")))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            if timer.elapsed() > polling_times * polling_interval {
                if !information_confirm {
                    control_channel_sender.send(AgentInformationPacket::new(information.clone())).await;
                } else {
                    let performance = Monitor::get_performance().await;
                    let performance_data = serde_json::to_vec(&performance)
                        .map_err(|err| error_entry!("Agent", "Unable to serialize data", format!("Err: {err}")))?;
                    control_channel_sender.send(PerformancePacket::new(performance_data)).await;
                }
                polling_times += 1;
            }
            select! {
                biased;
                packet = control_channel_receiver.agent_information_acknowledge_packet.recv() => {
                    let _ = packet
                        .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                    information_confirm = true;
                },
                packet = control_channel_receiver.performance_acknowledge_packet.recv() => {
                    let _ = packet
                        .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                    if !information_confirm {
                        Err(error_entry!("Agent", "Wrong packet delivery order"))?;
                    }
                    let agent = Self {
                        state: AgentState::None,
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
        Err(notice_entry!("Agent", "Control Channel timeout"))
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
            if agent.read().await.state == AgentState::Terminate {
                return;
            }
            if timeout_timer.elapsed() > timeout_duration {
                logging_notice!("Agent", "Control Channel timeout");
                break;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                let performance = Monitor::get_performance().await;
                match serde_json::to_vec(&performance) {
                    Ok(performance_data) => {
                        agent.write().await.control_channel_sender.send(PerformancePacket::new(performance_data)).await;
                    },
                    Err(err) => logging_error!("Agent", "Unable to serialize data", format!("Err: {err}")),
                }
                polling_times += 1;
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.performance_acknowledge_packet.recv() => {
                    if reply.is_some() {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_acknowledge_packet).await;
                        timeout_timer = Instant::now();
                    } else {
                        logging_notice!("Agent", "Channel has been closed");
                        break;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        agent.write().await.state = AgentState::Terminate;
    }

    async fn management(agent: Arc<RwLock<Agent>>) {
        loop {
            Self::refresh_state(agent.clone()).await;
            let state = agent.read().await.state;
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
        while agent.read().await.state != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::Terminate;
                return;
            }
            let mut agent = agent.write().await;
            select! {
                packet = agent.control_channel_receiver.control_packet.recv() => {
                    match packet {
                        Some(packet) => {
                            clear_unbounded_channel(&mut agent.control_channel_receiver.control_packet).await;
                            match serde_json::from_slice::<AgentState>(packet.as_data_byte()) {
                                Ok(state) => agent.state = state,
                                Err(err) => {
                                    logging_error!("Agent", "Unable to parse packet data", format!("Err: {err}"));
                                    continue;
                                },
                            }
                        },
                        None => {
                            logging_notice!("Agent", "Channel has been closed");
                            agent.state = AgentState::Terminate;
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
        let result = match Self::receive_task(agent.clone()).await {
            Ok(task_info) => {
                match Self::inference_task(task_info).await {
                    Ok(bounding_box) => Ok(bounding_box),
                    Err(entry) => {
                        logging_entry!(entry.clone());
                        Err(entry.message)
                    },
                }
            },
            Err(entry) => {
                logging_entry!(entry.clone());
                Err(entry.message)
            },
        };
        if let Err(entry) = Self::send_result(agent.clone(), result).await {
            logging_entry!(entry);
        }
    }

    async fn receive_task(agent: Arc<RwLock<Agent>>) -> Result<TaskInfo, LogEntry> {
        let task_info = Self::receive_task_info(agent.clone()).await?;
        let previous_task_uuid = agent.read().await.previous_task_uuid;
        let need_receive_model = if let Some(previous_task_uuid) = previous_task_uuid {
            previous_task_uuid != task_info.uuid
        } else {
            true
        };
        if need_receive_model {
            let model_folder = Path::new(".").join("SavedModel");
            Self::receive_file(agent.clone(), &model_folder).await?;
            agent.write().await.previous_task_uuid = Some(task_info.uuid);
        }
        let image_folder = Path::new(".").join("SavedFile");
        Self::receive_file(agent.clone(), &image_folder).await?;
        Ok(task_info)
    }

    async fn receive_task_info(agent: Arc<RwLock<Agent>>) -> Result<TaskInfo, LogEntry> {
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        let task_info = loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(notice_entry!("Agent", "Terminate. Interrupt current operation"))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(notice_entry!("Agent", "Data Channel timeout"))?;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    packet = data_channel_receiver.task_info_packet.recv() => {
                        let packet = packet
                            .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                        clear_unbounded_channel(&mut data_channel_receiver.task_info_packet).await;
                        break serde_json::from_slice::<TaskInfo>(packet.as_data_byte())
                            .map_err(|err| error_entry!("Agent", "Unable to parse packet data", format!("Err: {err}")))?;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent", "Data Channel is not ready"))?
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(TaskInfoAcknowledgePacket::new()).await;
        } else {
            Err(warning_entry!("Agent", "Data Channel is not ready"))?;
        }
        return Ok(task_info);
    }

    async fn receive_file(agent: Arc<RwLock<Agent>>, save_folder: &PathBuf) -> Result<(), LogEntry> {
        let file_header = Self::receive_file_header(agent.clone()).await?;
        let file_body = Self::receive_file_body(agent.clone(), &file_header).await?;
        Self::create_file(file_header, file_body, save_folder).await?;
        Ok(())
    }

    async fn receive_file_header(agent: Arc<RwLock<Agent>>) -> Result<FileHeader, LogEntry> {
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
            clear_unbounded_channel(&mut data_channel_receiver.file_header_packet).await;
        }
	    let file_header = loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(notice_entry!("Agent", "Terminate. Interrupt current operation"))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(notice_entry!("Agent", "Data Channel timeout"))?;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    packet = data_channel_receiver.file_header_packet.recv() => {
                        let packet = packet
                            .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                        clear_unbounded_channel(&mut data_channel_receiver.file_header_packet).await;
                        break serde_json::from_slice::<FileHeader>(packet.as_data_byte())
                            .map_err(|err| error_entry!("Agent", "Unable to parse packet data", format!("Err: {err}")))?;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent", "Data Channel is not ready"))?;
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(FileHeaderAcknowledgePacket::new()).await;
        } else {
            Err(warning_entry!("Agent", "Data Channel is not ready"))?;
        }
        return Ok(file_header);
    }

    async fn receive_file_body(agent: Arc<RwLock<Agent>>, file_header: &FileHeader) -> Result<Vec<Vec<u8>>, LogEntry> {
        let config = Config::now().await;
        let mut file_block: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut missing_blocks = Vec::new();
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(notice_entry!("Agent", "Terminate. Interrupt current operation"))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(notice_entry!("Agent", "Data Channel timeout"))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_body_packet.recv() => {
                        let packet = &packet
                            .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                        clear_unbounded_channel(&mut data_channel_receiver.file_body_packet).await;
                        timer = Instant::now();
                        let (sequence_bytes, file_body) = packet.data.split_at(mem::size_of::<usize>());
                        let sequence_bytes = sequence_bytes.try_into()
                            .map_err(|err| error_entry!("Agent", "Unable to parse packet data", format!("Err: {err}")))?;
                        let sequence_number = usize::from_be_bytes(sequence_bytes);
                        file_block.insert(sequence_number, Vec::from(file_body));
                        continue;
                    },
                    packet = data_channel_receiver.file_transfer_end_packet.recv() => {
                        let _ = &packet
                            .ok_or(notice_entry!("Agent", "Channel has been closed"))?;
                        timer = Instant::now();
                        for sequence_number in 0..file_header.packet_count {
                            if !file_block.contains_key(&sequence_number) {
                                missing_blocks.push(sequence_number);
                            }
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent", "Data Channel is not ready"))?;
            }
            if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                if missing_blocks.len() != 0_usize {
                    let missing_blocks = mem::take(&mut missing_blocks);
                    let result = FileTransferResult::new(Some(missing_blocks));
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|err| error_entry!("Agent", "Unable to serialize data", format!("Err: {err}")))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                } else {
                    let result = FileTransferResult::new(None);
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|err| error_entry!("Agent", "Unable to serialize data", format!("Err: {err}")))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                    let mut sorted_blocks: Vec<Vec<u8>> = Vec::with_capacity(file_header.packet_count);
                    for index in 0..file_header.packet_count {
                        if let Some(block) = file_block.remove(&index) {
                            sorted_blocks.push(block);
                        } else {
                            Err(error_entry!("Agent", "Missing file block"))?
                        }
                    }
                    return Ok(sorted_blocks);
                };
            } else {
                Err(warning_entry!("Agent", "Data Channel is not ready"))?;
            }
        }
    }

    async fn create_file(file_header: FileHeader, file_body: Vec<Vec<u8>>, saved_folder: &PathBuf) -> Result<(), LogEntry> {
        let saved_path = saved_folder.join(file_header.filename);
        let mut file = File::create(&saved_path).await
            .map_err(|err| error_entry!("Agent", "Unable create file", format!("File: {}, Err: {}", saved_path.display(), err)))?;
        for chunk in file_body {
            file.write_all(&chunk).await
                .map_err(|err| error_entry!("Agent", "Unable to write to file", format!("File: {}, Err: {}", saved_path.display(), err)))?;
        }
        Ok(())
    }

    async fn inference_task(task_info: TaskInfo) -> Result<Vec<BoundingBox>, LogEntry> {
        let model_path = Path::new(".").join("SavedModel").join(task_info.model_filename);
        let image_path = Path::new(".").join("SavedFile").join(task_info.image_filename);
        return match task_info.model_type {
            ModelType::Ultralytics =>  CalculateManager::ultralytics_inference(model_path, image_path).await,
            ModelType::YOLOv4 => CalculateManager::yolov4_inference(model_path, image_path).await,
            ModelType::YOLOv7 => CalculateManager::yolov7_inference(model_path, image_path).await,
        };
    }

    async fn send_result(agent: Arc<RwLock<Agent>>, result: Result<Vec<BoundingBox>, String>) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let result = TaskResult::new(result);
        let result_data = serde_json::to_vec(&result)
            .map_err(|err| error_entry!("Agent", "Unable to serialize data", format!("Err: {err}")))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(notice_entry!("Agent", "Terminate. Interrupt current operation"))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(notice_entry!("Agent", "Data Channel timeout"))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                    data_channel_sender.send(ResultPacket::new(result_data.clone())).await;
                } else {
                    Err(warning_entry!("Agent", "Data Channel is not ready"))?;
                }
                polling_times += 1;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.result_acknowledge_packet.recv() => {
                        if let Some(_) = packet {
                            clear_unbounded_channel(&mut data_channel_receiver.result_acknowledge_packet).await;
                            return Ok(());
                        } else {
                            Err(notice_entry!("Agent", "Channel has been closed"))?;
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!("Agent", "Data Channel is not ready"))?;
            }
        }
    }

    async fn idle(agent: Arc<RwLock<Agent>>, idle_duration: Duration) {
        let config = Config::now().await;
        let timer = Instant::now();
        loop {
            if agent.read().await.state == AgentState::Terminate {
                logging_notice!("Agent", "Terminate. Interrupt current operation");
                return;
            }
            if timer.elapsed() > idle_duration {
                return;
            }
            let mut agent = agent.write().await;
            if let Some(data_channel_receiver) = agent.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    _ = data_channel_receiver.alive_packet.recv() => clear_unbounded_channel(&mut data_channel_receiver.alive_packet).await,
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                logging_warning!("Agent", "Data Channel is not available.");
                return;
            }
            if let Some(data_channel_sender) = agent.data_channel_sender.as_mut() {
                data_channel_sender.send(AliveAcknowledgePacket::new()).await;
            } else {
                logging_warning!("Agent", "Data Channel is not available.");
                return;
            }
        }
    }

    #[allow(unused_assignments)]
    async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        let mut port: Option<u16> = None;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                logging_notice!("Agent", "Terminate. Interrupt current operation");
                return;
            }
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::Terminate;
                logging_notice!("Agent", "Control Channel timout.");
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
                                logging_error!("Agent", "Unable to parse packet data");
                                continue;
                            }
                        } else {
                            agent.state = AgentState::Terminate;
                            logging_notice!("Agent", "Channel has been closed");
                            return;
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
            if let Some(port) = port {
                let address = format!("{}:{}", config.management_address, port);
                match TcpStream::connect(&address).await {
                    Ok(tcp_stream) => {
                        let socket_stream = SocketStream::new(tcp_stream);
                        let (data_channel_sender, data_channel_receiver) = DataChannel::new(socket_stream);
                        let mut agent = agent.write().await;
                        agent.data_channel_sender = Some(data_channel_sender);
                        agent.data_channel_receiver = Some(data_channel_receiver);
                        logging_information!("Agent", "Data Channel created successfully");
                        return;
                    },
                    Err(err) => {
                        logging_error!("Agent", "Unable to establish connection", format!("Err: {err}"));
                        return;
                    },
                }
            } else {
                logging_error!("Agent", "Unknown error");
                return;
            }
        }
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        logging_information!("Agent", "Termination in progress");
        let mut agent = agent.write().await;
        agent.control_channel_sender.disconnect().await;
        agent.control_channel_receiver.disconnect().await;
        if let Some(data_channel_sender) = &mut agent.data_channel_sender {
            data_channel_sender.disconnect().await;
        }
        if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
            data_channel_receiver.disconnect().await;
        }
        logging_information!("Agent", "Termination complete");
    }
}
