use std::mem;
use uuid::Uuid;
use std::sync::Arc;
use tokio::fs::File;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::{fs, select};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use tokio::time::{sleep, Duration, Instant};
use crate::utils::logger::*;
use crate::utils::config::Config;
use crate::utils::port_pool::PortPool;
use crate::connection::packet::Packet;
use crate::utils::clear_unbounded_channel;
use crate::connection::channel::DataChannel;
use crate::connection::channel::ControlChannel;
use crate::management::task_manager::TaskManager;
use crate::management::utils::task_info::TaskInfo;
use crate::management::agent_manager::AgentManager;
use crate::management::utils::image_task::ImageTask;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::task_result::TaskResult;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::performance::Performance;
use crate::connection::packet::alive_packet::AlivePacket;
use crate::connection::socket::socket_stream::SocketStream;
use crate::management::utils::prevent_reenter::PreventReenter;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::management::utils::agent_information::AgentInformation;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::management::utils::file_transfer_result::FileTransferResult;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::packet::still_process_packet::StillProcessPacket;
use crate::connection::packet::control_packet::ControlPacket;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::packet::agent_information_acknowledge_packet::AgentInformationAcknowledgePacket;
use crate::connection::packet::performance_acknowledge_packet::PerformanceAcknowledgePacket;


pub struct Agent {
    uuid: Uuid,
    information: AgentInformation,
    idle_unused: Performance,
    realtime_usage: Performance,
    image_task: VecDeque<ImageTask>,
    previous_task_uuid: Option<Uuid>,
    control_channel_sender: ControlChannelSender,
    control_channel_receiver: ControlChannelReceiver,
    data_channel_sender: Option<DataChannelSender>,
    data_channel_receiver: Option<DataChannelReceiver>,
}

impl Agent {
    pub async fn new(uuid: Uuid, socket_stream: SocketStream) -> Result<Self, LogEntry> {
        let config = Config::now().await;
        let mut agent_information: Option<AgentInformation> = None;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(uuid, socket_stream);
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            select! {
                biased;
                reply = control_channel_receiver.agent_information_packet.recv() => {
                    let packet = &reply
                        .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                    clear_unbounded_channel(&mut control_channel_receiver.agent_information_packet).await;
                    let information = serde_json::from_slice::<AgentInformation>(packet.as_data_byte())
                        .map_err(|_| error_entry!("Agent: Unable to parse information."))?;
                    agent_information = Some(information);
                    control_channel_sender.send(AgentInformationAcknowledgePacket::new()).await;
                },
                reply = control_channel_receiver.performance_packet.recv() => {
                    let packet = &reply
                        .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                    clear_unbounded_channel(&mut control_channel_receiver.performance_packet).await;
                    let realtime_usage = serde_json::from_slice::<Performance>(packet.as_data_byte())
                        .map_err(|_| error_entry!("Agent: Unable to parse performance."))?;
                    let information = agent_information
                        .ok_or(error_entry!("Agent: Agent information not ready."))?;
                    control_channel_sender.send(PerformanceAcknowledgePacket::new()).await;
                    let residual_usage = Performance::calc_residual_usage(&information, &realtime_usage);
                    let agent = Self {
                        uuid,
                        information,
                        idle_unused: residual_usage,
                        realtime_usage,
                        image_task: VecDeque::new(),
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
        Err(info_entry!("Agent: Fail create instance. Control Channel timeout."))
    }

    pub async fn add_task(agent: Arc<RwLock<Agent>>, image_task: ImageTask) {
        agent.write().await.image_task.push_back(image_task);
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        let for_performance = agent.clone();
        let for_management = agent;
        tokio::spawn(async move {
            Agent::performance(for_performance).await;
        });
        tokio::spawn(async move {
            Agent::management(for_management).await;
        });
    }

    async fn performance(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if AgentManager::get_state(uuid).await == AgentState::Terminate {
                return;
            }
            if timer.elapsed() > timeout_duration {
                logging_warning!(uuid, "Agent: Control Channel timeout.");
                break;
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.performance_packet.recv() => {
                    if let Some(packet) = &reply {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_packet).await;
                        if let Ok(performance) = serde_json::from_slice::<Performance>(packet.as_data_byte()) {
                            agent.realtime_usage = performance;
                            agent.control_channel_sender.send(PerformanceAcknowledgePacket::new()).await;
                            timer = Instant::now();
                        } else {
                            logging_error!(uuid, "Agent: Unable to parse performance.");
                            continue;
                        }
                    } else {
                        logging_warning!(uuid, "Agent: Channel has been closed.");
                        break;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        AgentManager::store_state(uuid, AgentState::Terminate).await;
    }

    async fn management(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        loop {
            let config = Config::now().await;
            let state = AgentManager::get_state(uuid).await;
            match state {
                AgentState::CreateDataChannel => {
                    Self::control_agent(agent.clone(), AgentState::CreateDataChannel).await;
                    Self::create_data_channel(agent.clone()).await;
                    AgentManager::store_state(uuid, AgentState::None).await;
                },
                AgentState::Terminate => {
                    Self::control_agent(agent.clone(), AgentState::Terminate).await;
                    Self::terminate(agent).await;
                    return;
                },
                _ => {
                    if let Some(mut image_task) = agent.write().await.image_task.pop_front() {
                        let state = AgentState::ProcessTask;
                        AgentManager::store_state(uuid, state).await;
                        Self::control_agent(agent.clone(), state).await;
                        let success = Self::process_task(agent.clone(), &mut image_task).await;
                        TaskManager::submit_image_task(image_task, success).await;
                    } else {
                        let state = AgentState::Idle(config.agent_idle_duration);
                        AgentManager::store_state(uuid, state).await;
                        Self::control_agent(agent.clone(), state).await;
                        Self::idle(agent.clone()).await;
                    }
                },
            }
        }
    }

    async fn control_agent(agent: Arc<RwLock<Agent>>, state: AgentState) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        if let Ok(control_state_data) = serde_json::to_vec(&state) {
            loop {
                if timer.elapsed() > timeout_duration {
                    logging_warning!(uuid, "Agent: Transfer control packet timeout.");
                    break;
                }
                if timer.elapsed() > polling_times * polling_interval {
                    let agent = agent.write().await;
                    agent.control_channel_sender.send(ControlPacket::new(control_state_data.clone())).await;
                    polling_times += 1;
                }
                let mut agent = agent.write().await;
                select! {
                    reply = agent.control_channel_receiver.control_acknowledge_packet.recv() => {
                        if reply.is_none() {
                            logging_warning!(uuid, "Agent: Channel has been closed.");
                            break;
                        }
                        clear_unbounded_channel(&mut agent.control_channel_receiver.control_acknowledge_packet).await;
                        return;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)).await => continue,
                }
            }
            AgentManager::store_state(uuid, AgentState::Terminate).await;
        } else {
            logging_error!(uuid, "Agent: Unable to serialized control state data.");
        }
    }

    async fn process_task(agent: Arc<RwLock<Agent>>, image_task: &mut ImageTask) -> bool {
        let uuid = agent.read().await.uuid;
        if let Err(entry) = Self::transfer_task(agent.clone(), image_task).await {
            logging_entry!(uuid, entry);
            return false;
        }
        if let Err(entry) = Self::waiting_result(agent, image_task).await {
            logging_entry!(uuid, entry);
            return false;
        }
        true
    }

    async fn transfer_task(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        if agent.write().await.data_channel_sender.is_some() {
            let need_transfer_model = if let Some(last_task) = &agent.read().await.previous_task_uuid {
                image_task.task_uuid != *last_task
            } else {
                true
            };
            Agent::transfer_task_info(agent.clone(), &image_task).await?;
            if need_transfer_model {
                Agent::transfer_file(agent.clone(), &image_task.model_filename, &image_task.model_filepath).await?;
            }
            Agent::transfer_file(agent.clone(), &image_task.image_filename, &image_task.image_filepath).await?;
        } else {
            AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
            Err(warning_entry!("Agent: Data Channel is not available."))?
        }
        Ok(())
    }

    async fn transfer_task_info(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let task_info = TaskInfo::new(image_task.task_uuid, image_task.model_filename.clone(), image_task.model_type);
        let task_info_data = serde_json::to_vec(&task_info)
            .map_err(|_| error_entry!("Agent: Unable to serialized task info data."))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while AgentManager::get_state(uuid).await != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                Err(warning_entry!("Agent: Transfer task info timeout."))?
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                match agent.data_channel_sender.as_mut() {
                    Some(data_channel_sender) => {
                        data_channel_sender.send(TaskInfoPacket::new(task_info_data.clone())).await;
                        polling_times += 1;
                    },
                    None => {
                        AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                        Err(warning_entry!("Agent: Data Channel is not available."))?
                    },
                }
            }
            let mut agent = agent.write().await;
            match agent.data_channel_receiver.as_mut() {
                Some(data_channel_receiver) => {
                    select! {
                        reply = data_channel_receiver.task_info_reply_packet.recv() => {
                            reply.ok_or(warning_entry!("Agent: Channel has been closed."))?;
                            clear_unbounded_channel(&mut data_channel_receiver.task_info_reply_packet).await;
                            return Ok(())
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                },
                None => {
                    AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                    Err(warning_entry!("Agent: Data Channel is not available."))?
                },
            }
        }
        Err(info_entry!("Agent: Terminating. Transfer task info cancel."))
    }

    async fn transfer_file(agent: Arc<RwLock<Agent>>, filename: &String, filepath: &PathBuf) -> Result<(), LogEntry> {
        Self::transfer_file_header(agent.clone(), filename, filepath).await?;
        let sent_packets = Self::transfer_file_body(agent.clone(), filepath).await?;
        Self::retransmit_file(agent, sent_packets).await
    }

    async fn transfer_file_header(agent: Arc<RwLock<Agent>>, filename: &String, filepath: &PathBuf) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let filesize = fs::metadata(&filepath).await
            .map_err(|err| error_entry!(format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display())))?
            .len();
        if AgentManager::get_state(uuid).await == AgentState::Terminate {
            Err(info_entry!("Agent: Terminating. File transfer cancel."))?;
        }
        let mut agent = agent.write().await;
        match agent.data_channel_sender.as_mut() {
            Some(data_channel_sender) => {
                let file_header = serde_json::to_vec(&FileHeader::new(filename.clone(), filesize as usize))
                    .map_err(|_| error_entry!("Agent: Unable to serialized file header data."))?;
                data_channel_sender.send(FileHeaderPacket::new(file_header)).await;
            },
            None => {
                AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                Err(warning_entry!("Agent: Data Channel is not available."))?
            },
        }
        Ok(())
    }

    async fn transfer_file_body(agent: Arc<RwLock<Agent>>, filepath: &PathBuf) -> Result<Vec<Vec<u8>>, LogEntry> {
        let uuid = agent.read().await.uuid;
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut sent_packets = Vec::new();
        let mut file = File::open(filepath.clone()).await
            .map_err(|err| error_entry!(format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display())))?;
        loop {
            if AgentManager::get_state(uuid).await == AgentState::Terminate {
                Err(info_entry!("Agent: Terminating. File transfer cancel."))?;
            }
            let bytes_read = file.read(&mut buffer).await
                .map_err(|_| error_entry!(format!("Agent: An error occurred while reading file {filepath}.", filepath = filepath.display())))?;
            if bytes_read == 0 {
                break;
            }
            let mut data = sequence_number.to_be_bytes().to_vec();
            data.extend_from_slice(&buffer[..bytes_read]);
            let mut agent = agent.write().await;
            match agent.data_channel_sender.as_mut() {
                Some(data_channel_sender) => {
                    data_channel_sender.send(FileBodyPacket::new(data.clone())).await;
                    sent_packets.push(data);
                    sequence_number += 1;
                },
                None => {
                    AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                    Err(warning_entry!("Agent: Data Channel is not available."))?
                },
            }
        }
        Ok(sent_packets)
    }

    #[allow(unused_assignments)]
    async fn retransmit_file(agent: Arc<RwLock<Agent>>, sent_packets: Vec<Vec<u8>>) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        let mut require_resend = Vec::new();
        while time.elapsed() < timeout_duration {
            {
                if AgentManager::get_state(uuid).await == AgentState::Terminate {
                    Err(info_entry!("Agent: Terminating. File transfer cancel."))?;
                }
                let mut agent = agent.write().await;
                match agent.data_channel_receiver.as_mut() {
                    Some(data_channel_receiver) => {
                        select! {
                            biased;
                            reply = data_channel_receiver.file_transfer_reply_packet.recv() => {
                                let packet = reply
                                    .ok_or(warning_entry!("Agent: Channel has been closed."))?;
                                clear_unbounded_channel(&mut data_channel_receiver.file_transfer_reply_packet).await;
                                let file_transfer_result = serde_json::from_slice::<FileTransferResult>(packet.as_data_byte())
                                    .map_err(|_| error_entry!("Agent: Unable to parse file transfer result."))?;
                                match file_transfer_result.into() {
                                    Some(missing_chunks) => require_resend = missing_chunks,
                                    None => return Ok(()),
                                }
                            },
                            _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                        }
                    },
                    None => {
                        AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                        Err(warning_entry!("Agent: Data Channel is not available."))?
                    },
                }
            }
            for missing_chunk in &require_resend {
                if let Some(data) = sent_packets.get(*missing_chunk) {
                    if AgentManager::get_state(uuid).await == AgentState::Terminate {
                        Err(info_entry!("Agent: Terminating. File transfer cancel."))?;
                    }
                    let mut agent = agent.write().await;
                    match agent.data_channel_sender.as_mut() {
                        Some(data_channel_sender) => data_channel_sender.send(FileBodyPacket::new(data.clone())).await,
                        None => {
                            AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                            Err(warning_entry!("Agent: Data Channel is not available."))?
                        },
                    }
                } else {
                    Err(error_entry!("Agent: File block missing."))?
                }
            }
        }
        Err(warning_entry!("Agent: File transfer timeout."))
    }

    async fn waiting_result(agent: Arc<RwLock<Agent>>, image_task: &mut ImageTask) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut polling_times = 0_u32;
        let polling_timer = Instant::now();
        let polling_interval = Duration::from_millis(config.polling_interval);
        let mut timeout_timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if AgentManager::get_state(uuid).await == AgentState::Terminate {
                Err(info_entry!("Agent: Terminating. Interrupt task processing."))?;
            }
            if timeout_timer.elapsed() > timeout_duration {
                AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                Err(warning_entry!("Agent: Data Channel timeout."))?;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                match &mut agent.write().await.data_channel_sender {
                    Some(data_channel_sender) => data_channel_sender.send(StillProcessPacket::new()).await,
                    None => {
                        AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                        Err(warning_entry!("Agent: Data Channel is not available."))?;
                    }
                }
                polling_times += 1;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    biased;
                    reply = data_channel_receiver.still_process_reply_packet.recv() => {
                        if reply.is_some() {
                            clear_unbounded_channel(&mut data_channel_receiver.still_process_reply_packet).await;
                            timeout_timer = Instant::now();
                        } else {
                            AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                            Err(warning_entry!("Agent: Channel has been closed."))?;
                        }
                    },
                    reply = data_channel_receiver.result_packet.recv() => {
                        if let Some(reply_packet) = &reply {
                            clear_unbounded_channel(&mut data_channel_receiver.result_packet).await;
                            if let Ok(task_result) = serde_json::from_slice::<TaskResult>(reply_packet.as_data_byte()) {
                                match task_result.into() {
                                    Ok(bounding_box) => {
                                        image_task.bounding_boxes = bounding_box;
                                        break;
                                    },
                                    Err(err) => {
                                        Err(error_entry!(format!("Agent: An error occurred while processing.\nReason: {err}")))?;
                                    },
                                }
                            } else {
                                Err(error_entry!("Agent: Unable to parse task result."))?;
                            }
                        } else {
                            AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                            Err(warning_entry!("Agent: Channel has been closed."))?;
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                Err(warning_entry!("Agent: Data Channel is not available."))?;
            }
        }
        Ok(())
    }

    async fn idle(agent: Arc<RwLock<Agent>>) {
        if let Some(image_task) = TaskManager::steal_task(agent.clone()).await {
            Agent::add_task(agent.clone(), image_task).await;
        } else {
            let uuid = agent.read().await.uuid;
            let config = Config::now().await;
            {
                let mut agent = agent.write().await;
                agent.idle_unused = Performance::calc_residual_usage(&agent.information, &agent.realtime_usage);
            }
            let timer = Instant::now();
            let mut polling_times = 0_u32;
            let polling_interval = Duration::from_millis(config.polling_interval);
            let idle_duration = Duration::from_secs(config.agent_idle_duration);
            let mut timeout_timer = Instant::now();
            let timeout_duration = Duration::from_secs(config.control_channel_timeout);
            while AgentManager::get_state(uuid).await != AgentState::Terminate && timer.elapsed() <= idle_duration {
                if timeout_timer.elapsed() > timeout_duration {
                    AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                    logging_warning!(uuid, "Agent: Data Channel timeout.");
                    return;
                }
                if timer.elapsed() > polling_times * polling_interval {
                    if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
                        data_channel_sender.send(AlivePacket::new()).await
                    } else {
                        AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                        logging_warning!("Agent: Data Channel is not available.");
                        return;
                    }
                    polling_times += 1;
                }
                if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                    select! {
                        biased;
                        reply = data_channel_receiver.alive_reply_packet.recv() => {
                            if reply.is_some() {
                                clear_unbounded_channel(&mut data_channel_receiver.alive_reply_packet).await;
                                timeout_timer = Instant::now();
                            } else {
                                AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                                logging_warning!(uuid, "Agent: Channel has been closed.");
                                return;
                            }
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                } else {
                    AgentManager::store_state(uuid, AgentState::CreateDataChannel).await;
                    logging_warning!(uuid, "Agent: Data Channel is not available.");
                    return;
                }
            }
        }
    }

    async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        match Self::create_listener(agent.clone()).await {
            Ok((listener, port)) => {
                if let Err(entry) = Self::accept_connection(agent, listener, port).await {
                    logging_entry!(entry);
                }
            },
            Err(entry) => logging_entry!(entry),
        }
    }

    async fn create_listener(agent: Arc<RwLock<Agent>>) -> Result<(TcpListener, u16), LogEntry> {
        let uuid = agent.read().await.uuid;
        loop {
            if AgentManager::get_state(uuid).await == AgentState::Terminate {
                return Err(info_entry!("Agent: Terminating. Cancel creation of Data Channel."));
            }
            let port = PortPool::allocate_port().await
                .ok_or(error_entry!("Agent: No available port for Data Channel"))?;
            match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => break Ok((listener, port)),
                Err(err) => {
                    PortPool::free_port(port).await;
                    Err(error_entry!(format!("Agent: Port binding failed.\nReason: {err}")))?;
                }
            }
        }
    }

    async fn accept_connection(agent: Arc<RwLock<Agent>>, listener: TcpListener, port: u16) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let (tcp_stream, _) = loop {
            if AgentManager::get_state(uuid).await == AgentState::Terminate {
                PortPool::free_port(port).await;
                Err(info_entry!("Agent: Terminating. Cancel creation of Data Channel."))?
            }
            if timer.elapsed() > timeout_duration {
                PortPool::free_port(port).await;
                Err(warning_entry!("Agent: Create Data Channel timeout."))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                agent.write().await.control_channel_sender.send(DataChannelPortPacket::new(port)).await;
                polling_times += 1;
            }
            select! {
                biased;
                connection = listener.accept() => {
                    match connection {
                        Ok(connection) => break connection,
                        Err(err) => Err(error_entry!(format!("Agent: Failed to establish connection.\nReason: {}", err)))?
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        };
        let socket_stream = SocketStream::new(tcp_stream);
        let (data_channel_sender, data_channel_receiver) = DataChannel::new(uuid, socket_stream);
        let mut agent = agent.write().await;
        agent.data_channel_sender = Some(data_channel_sender);
        agent.data_channel_receiver = Some(data_channel_receiver);
        logging_info!(uuid, "Agent: Create Data channel successfully.");
        Ok(())
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        static TERMINATING_PROCESSING: AtomicBool = AtomicBool::new(false);
        let prevent_reenter = PreventReenter::new(&TERMINATING_PROCESSING);
        if prevent_reenter.is_none() {
            return;
        }
        let uuid = agent.read().await.uuid;
        logging_info!(uuid, "Agent: Terminating agent.");
        let image_task = {
            let mut agent = agent.write().await;
            agent.control_channel_sender.disconnect().await;
            agent.control_channel_receiver.disconnect().await;
            if let Some(data_channel_sender) = &mut agent.data_channel_sender {
                data_channel_sender.disconnect().await;
            }
            if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
                data_channel_receiver.disconnect().await;
            }
            mem::take(&mut agent.image_task)
        };
        TaskManager::redistribute_task(image_task).await;
        AgentManager::remove_agent(uuid).await;
        logging_info!(uuid, "Agent: Termination complete.");
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn agent_information(&self) -> AgentInformation {
        self.information.clone()
    }

    pub fn idle_unused(&self) -> Performance {
        self.idle_unused.clone()
    }

    pub fn realtime_usage(&self) -> Performance {
        self.realtime_usage.clone()
    }

    pub fn image_tasks(&mut self) -> &mut VecDeque<ImageTask> {
        &mut self.image_task
    }
}
