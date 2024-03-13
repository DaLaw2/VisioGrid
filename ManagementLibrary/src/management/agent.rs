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
use tokio::time::{sleep, Duration, Instant};
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
use crate::management::utils::task_result::TaskResult;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::performance::Performance;
use crate::utils::logger::{Logger, LogLevel, LogEntry};
use crate::management::utils::confirm_type::ConfirmType;
use crate::connection::packet::alive_packet::AlivePacket;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::packet::confirm_packet::ConfirmPacket;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::management::utils::agent_information::AgentInformation;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::management::utils::file_transfer_result::FileTransferResult;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::packet::still_process_packet::StillProcessPacket;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;

pub struct Agent {
    uuid: Uuid,
    terminate: bool,
    information: AgentInformation,
    idle_unused: Performance,
    realtime_usage: Performance,
    image_task: VecDeque<ImageTask>,
    previous_task: Option<ImageTask>,
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
                        .ok_or(LogEntry::new(LogLevel::INFO, "Agent: Channel has been closed.".to_string()))?;
                    clear_unbounded_channel(&mut control_channel_receiver.agent_information_packet).await;
                    let information = serde_json::from_slice::<AgentInformation>(packet.as_data_byte())
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to parse information.".to_string()))?;
                    agent_information = Some(information);
                    let confirm = serde_json::to_vec(&ConfirmType::ReceiveAgentInformationSuccess)
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized confirm type.".to_string()))?;
                    control_channel_sender.send(ConfirmPacket::new(confirm)).await;
                },
                reply = control_channel_receiver.performance_packet.recv() => {
                    let packet = &reply
                        .ok_or(LogEntry::new(LogLevel::INFO, "Agent: Channel has been closed.".to_string()))?;
                    clear_unbounded_channel(&mut control_channel_receiver.performance_packet).await;
                    let realtime_usage = serde_json::from_slice::<Performance>(packet.as_data_byte())
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to parse performance.".to_string()))?;
                    let information = agent_information
                        .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Agent information not ready.".to_string()))?;
                    let confirm = serde_json::to_vec(&ConfirmType::ReceivePerformanceSuccess)
                        .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized confirm data.".to_string()))?;
                    control_channel_sender.send(ConfirmPacket::new(confirm)).await;
                    let residual_usage = Performance::calc_residual_usage(&information, &realtime_usage);
                    let agent = Self {
                        uuid,
                        terminate: false,
                        information,
                        idle_unused: residual_usage,
                        realtime_usage,
                        image_task: VecDeque::new(),
                        previous_task: None,
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
        Err(LogEntry::new(LogLevel::INFO, "Agent: Fail create instance. Connection Channel timeout.".to_string()))
    }

    pub async fn add_task(agent: Arc<RwLock<Agent>>, image_task: ImageTask) {
        agent.write().await.image_task.push_back(image_task);
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        if let Err(entry) = Agent::create_data_channel(agent.clone()).await {
            let uuid = agent.read().await.uuid;
            Logger::add_agent_log_entry(uuid, entry).await
        }
        let for_performance = agent.clone();
        let for_task_management = agent;
        tokio::spawn(async move {
            Agent::performance(for_performance).await;
        });
        tokio::spawn(async move {
            Agent::task_management(for_task_management).await;
        });
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Terminating agent.".to_string()).await;
        let image_task = {
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
            mem::take(&mut agent.image_task)
        };
        TaskManager::redistribute_task(image_task).await;
        AgentManager::remove_agent(uuid).await;
        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Termination complete.".to_string()).await;
    }

    async fn performance(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while !agent.read().await.terminate {
            if timer.elapsed() > timeout_duration {
                Logger::add_agent_log(uuid, LogLevel::WARNING, "Agent: Control Channel timeout.".to_string()).await;
                Agent::terminate(agent).await;
                return;
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.performance_packet.recv() => {
                    if let Some(packet) = &reply {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_packet).await;
                        if let Ok(performance) = serde_json::from_slice::<Performance>(packet.as_data_byte()) {
                            if let Ok(confirm) = serde_json::to_vec(&ConfirmType::ReceivePerformanceSuccess) {
                                agent.realtime_usage = performance;
                                agent.control_channel_sender.send(ConfirmPacket::new(confirm)).await;
                                timer = Instant::now();
                            } else {
                                Logger::add_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to serialized confirm data.".to_string()).await;
                                continue;
                            }
                        } else {
                            Logger::add_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse performance.".to_string()).await;
                            continue;
                        }
                    } else {
                        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                        return;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
    }

    #[allow(unused_assignments)]
    async fn task_management(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut data_channel_available = true;
        while !agent.read().await.terminate {
            if let Some(mut image_task) = agent.write().await.image_task.pop_front() {
                if let Err(entry) = Agent::transfer_task(agent.clone(), &image_task).await {
                    Logger::add_agent_log_entry(uuid, entry).await;
                    TaskManager::submit_image_task(image_task, false).await;
                    continue;
                }
                let mut success = false;
                let mut polling_times = 0_u32;
                let polling_timer = Instant::now();
                let polling_interval = Duration::from_millis(config.polling_interval);
                let mut timeout_timer = Instant::now();
                let timeout_duration = Duration::from_secs(config.control_channel_timeout);
                loop {
                    if agent.read().await.terminate {
                        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Terminating. Interrupt task processing.".to_string()).await;
                        break;
                    }
                    if timeout_timer.elapsed() > timeout_duration {
                        Logger::add_agent_log(uuid, LogLevel::WARNING, "Agent: Data Channel timeout.".to_string()).await;
                        TaskManager::submit_image_task(image_task, false).await;
                        Agent::terminate(agent.clone()).await;
                        return;
                    }
                    if polling_timer.elapsed() > polling_times * polling_interval {
                        match &mut agent.write().await.data_channel_sender {
                            Some(data_channel_sender) => data_channel_sender.send(StillProcessPacket::new()).await,
                            None => {
                                data_channel_available = false;
                                break;
                            },
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
                                    Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                                    return;
                                }
                            },
                            reply = data_channel_receiver.result_packet.recv() => {
                                if let Some(reply_packet) = &reply {
                                    clear_unbounded_channel(&mut data_channel_receiver.result_packet).await;
                                    if let Ok(task_result) = serde_json::from_slice::<TaskResult>(reply_packet.as_data_byte()) {
                                        match task_result.into() {
                                            Ok(bounding_box) => {
                                                image_task.bounding_boxes = bounding_box;
                                                success = true;
                                            },
                                            Err(err) => Logger::add_agent_log(uuid, LogLevel::ERROR, format!("Agent: An error occurred while processing.\nReason: {err}")).await,
                                        }
                                    } else {
                                        Logger::add_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse task result.".to_string()).await;
                                    }
                                    break;
                                } else {
                                    Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                                    return;
                                }
                            },
                            _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                        }
                    } else {
                        data_channel_available = false;
                        break;
                    }
                }
                TaskManager::submit_image_task(image_task, success).await;
            } else {
                if let Some(image_task) = TaskManager::steal_task(agent.clone()).await {
                    Agent::add_task(agent.clone(), image_task).await
                } else {
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
                    while !agent.read().await.terminate && timer.elapsed() <= idle_duration {
                        if timeout_timer.elapsed() > timeout_duration {
                            Logger::add_agent_log(uuid, LogLevel::WARNING, "Agent: Data Channel timeout.".to_string()).await;
                            Agent::terminate(agent.clone()).await;
                            return;
                        }
                        if timer.elapsed() > polling_times * polling_interval {
                            if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
                                data_channel_sender.send(AlivePacket::new()).await
                            } else {
                                data_channel_available = false;
                                break;
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
                                        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                                        return;
                                    }
                                },
                                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                            }
                        } else {
                            data_channel_available = false;
                            break;
                        }
                    }
                }
            }
            if !data_channel_available {
                if let Err(entry) = Agent::create_data_channel(agent.clone()).await {
                    Logger::add_agent_log_entry(uuid, entry).await;
                }
            }
        }
    }

    async fn create_data_channel(agent: Arc<RwLock<Agent>>) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let (listener, port) = loop {
            if agent.read().await.terminate {
                Err(LogEntry::new(LogLevel::INFO, "Agent: Terminating. Cancel creation of Data Channel.".to_string()))?;
            }
            let port = PortPool::allocate_port().await
                .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: No available port for Data Channel".to_string()))?;
            match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => break (listener, port),
                Err(err) => {
                    PortPool::free_port(port).await;
                    Err(LogEntry::new(LogLevel::ERROR, format!("Agent: Port binding failed.\nReason: {err}")))?;
                },
            }
        };
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let (tcp_stream, _) = loop {
            if agent.read().await.terminate {
                PortPool::free_port(port).await;
                Err(LogEntry::new(LogLevel::INFO, "Agent: Terminating. Cancel creation of Data Channel.".to_string()))?;
            }
            if timer.elapsed() > timeout_duration {
                PortPool::free_port(port).await;
                Err(LogEntry::new(LogLevel::ERROR, "Agent: Create Data Channel timeout.".to_string()))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                agent.write().await.control_channel_sender.send(DataChannelPortPacket::new(port)).await;
                polling_times += 1;
            }
            select! {
                biased;
                connection = listener.accept() => {
                    if let Ok(connection) = connection {
                        break connection;
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
        Logger::add_agent_log(uuid, LogLevel::INFO, "Agent: Create Data channel successfully.".to_string()).await;
        Ok(())
    }

    async fn transfer_task(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), LogEntry> {
        if agent.write().await.data_channel_sender.is_some() {
            let should_transfer_model = if let Some(last_task) = &agent.read().await.previous_task {
                image_task.task_uuid != last_task.task_uuid
            } else {
                true
            };
            Agent::transfer_task_info(agent.clone(), &image_task).await?;
            if should_transfer_model {
                Agent::transfer_file(agent.clone(), &image_task.model_filename, &image_task.model_filepath).await?;
            }
            Agent::transfer_file(agent.clone(), &image_task.image_filename, &image_task.image_filepath).await?;
        } else {
            Agent::create_data_channel(agent.clone()).await?;
            Err(LogEntry::new(LogLevel::WARNING, "Agent: Data Channel is not available.".to_string()))?
        }
        Ok(())
    }

    async fn transfer_task_info(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let task_info = TaskInfo::new(image_task.task_uuid.clone(), image_task.model_filename.clone(), image_task.model_type);
        let task_info_data = serde_json::to_vec(&task_info)
            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized task info data.".to_string()))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() < timeout_duration {
            if agent.read().await.terminate {
                Err(LogEntry::new(LogLevel::INFO, "Agent: Terminating. Transfer task info cancel.".to_string()))?
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                let data_channel_sender = agent.data_channel_sender.as_mut()
                    .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data Channel is not available.".to_string()))?;
                data_channel_sender.send(TaskInfoPacket::new(task_info_data.clone())).await;
                polling_times += 1;
            }
            let mut agent = agent.write().await;
            let data_channel_receiver = agent.data_channel_receiver.as_mut()
                .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data Channel is not available.".to_string()))?;
            select! {
                reply = data_channel_receiver.task_info_reply_packet.recv() => {
                    reply.ok_or(LogEntry::new(LogLevel::INFO, "Agent: Channel has been closed.".to_string()))?;
                    clear_unbounded_channel(&mut data_channel_receiver.task_info_reply_packet).await;
                    return Ok(())
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        Err(LogEntry::new(LogLevel::ERROR, "Agent: Task Info retransmission limit reached.".to_string()))
    }

    #[allow(unused_assignments)]
    async fn transfer_file(agent: Arc<RwLock<Agent>>, filename: &String, filepath: &PathBuf) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let filesize = fs::metadata(&filepath).await
            .map_err(|err| LogEntry::new(LogLevel::ERROR, format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display())))?
            .len();
        {
            let mut agent = agent.write().await;
            let data_channel_sender = agent.data_channel_sender.as_mut()
                .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data channel is not available.".to_string()))?;
            let file_header = serde_json::to_vec(&FileHeader::new(filename.clone(), filesize as usize))
                .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to serialized file header data.".to_string()))?;
            data_channel_sender.send(FileHeaderPacket::new(file_header)).await;
        }
        let file = File::open(filepath.clone()).await;
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut sent_packets = Vec::new();
        let mut file = file
            .map_err(|err| LogEntry::new(LogLevel::ERROR, format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display())))?;
        loop {
            if agent.read().await.terminate {
                Err(LogEntry::new(LogLevel::INFO, "Agent: Terminating. File transfer cancel.".to_string()))?;
            }
            let bytes_read = file.read(&mut buffer).await
                .map_err(|_| LogEntry::new(LogLevel::ERROR, format!("Agent: An error occurred while reading file {filepath}.", filepath = filepath.display())))?;
            if bytes_read == 0 {
                break;
            }
            let mut data = sequence_number.to_be_bytes().to_vec();
            data.extend_from_slice(&buffer[..bytes_read]);
            let mut agent = agent.write().await;
            let data_channel_sender = agent.data_channel_sender.as_mut()
                .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data channel is not available.".to_string()))?;
            data_channel_sender.send(FileBodyPacket::new(data.clone())).await;
            sent_packets.push(data);
            sequence_number += 1;
        }
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        let mut require_resend = Vec::new();
        while time.elapsed() < timeout_duration {
            if agent.read().await.terminate {
                Err(LogEntry::new(LogLevel::INFO, "Agent: Terminating. File transfer cancel.".to_string()))?;
            }
            {
                let mut agent = agent.write().await;
                let data_channel_receiver = agent.data_channel_receiver.as_mut()
                    .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data channel is not available.".to_string()))?;
                select! {
                    biased;
                    reply = data_channel_receiver.file_transfer_reply_packet.recv() => {
                        let packet = reply
                            .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Channel has been closed.".to_string()))?;
                        clear_unbounded_channel(&mut data_channel_receiver.file_transfer_reply_packet).await;
                        let file_transfer_result = serde_json::from_slice::<FileTransferResult>(packet.as_data_byte())
                            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Agent: Unable to parse file transfer result.".to_string()))?;
                        match file_transfer_result.into() {
                            Some(missing_chunks) => require_resend = missing_chunks,
                            None => return Ok(()),
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
            for missing_chunk in &require_resend {
                if let Some(data) = sent_packets.get(*missing_chunk) {
                    let mut agent = agent.write().await;
                    let data_channel_sender = agent.data_channel_sender.as_mut()
                        .ok_or(LogEntry::new(LogLevel::ERROR, "Agent: Data channel is not available.".to_string()))?;
                    data_channel_sender.send(FileBodyPacket::new(data.clone())).await;
                }
            }
        }
        Err(LogEntry::new(LogLevel::ERROR, "Agent: File transfer timeout.".to_string()))
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn agent_information(&self) -> &AgentInformation {
        &self.information
    }

    pub fn idle_unused(&self) -> &Performance {
        &self.idle_unused
    }

    pub fn realtime_usage(&self) -> &Performance {
        &self.realtime_usage
    }

    pub fn image_tasks(&mut self) -> &mut VecDeque<ImageTask> {
        &mut self.image_task
    }

    pub fn mut_realtime_usage(&mut self) -> &mut Performance {
        &mut self.realtime_usage
    }
}
