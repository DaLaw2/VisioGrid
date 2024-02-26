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
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::channel::ControlChannel;
use crate::management::task_manager::TaskManager;
use crate::management::utils::task_info::TaskInfo;
use crate::management::agent_manager::AgentManager;
use crate::management::utils::image_task::ImageTask;
use crate::management::utils::task_result::TaskResult;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::performance::Performance;
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
    pub async fn new(uuid: Uuid, socket_stream: SocketStream) -> Option<Self> {
        let config = Config::now().await;
        let mut agent_information: Option<AgentInformation> = None;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(uuid, socket_stream);
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            select! {
                biased;
                reply = control_channel_receiver.agent_information_packet.recv() => {
                    return if let Some(packet) = &reply {
                        clear_unbounded_channel(&mut control_channel_receiver.agent_information_packet).await;
                        if let Ok(information) = serde_json::from_slice::<AgentInformation>(packet.as_data_byte()) {
                            agent_information = Some(information);
                            if let Ok(confirm) = serde_json::to_vec(&ConfirmType::ReceiveAgentInformationSuccess) {
                                control_channel_sender.send(ConfirmPacket::new(confirm)).await;
                                continue;
                            } else {
                                Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to serialized confirm type.".to_string()).await;
                                None
                            }
                        } else {
                            Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse information.".to_string()).await;
                            None
                        }
                    } else {
                        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                        None
                    }
                },
                reply = control_channel_receiver.performance_packet.recv() => {
                    return if let Some(packet) = &reply {
                        clear_unbounded_channel(&mut control_channel_receiver.performance_packet).await;
                        if let Ok(realtime_usage) = serde_json::from_slice::<Performance>(packet.as_data_byte()) {
                            if let Some(information) = agent_information {
                                if let Ok(confirm) = serde_json::to_vec(&ConfirmType::ReceivePerformanceSuccess) {
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
                                    Some(agent)
                                } else {
                                    Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to serialized confirm data.".to_string()).await;
                                    None
                                }
                            } else {
                                Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Agent information not ready.".to_string()).await;
                                None
                            }
                        } else {
                            Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse performance.".to_string()).await;
                            None
                        }
                    } else {
                        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                        None
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        None
    }

    pub async fn add_task(agent: Arc<RwLock<Agent>>, image_task: ImageTask) {
        agent.write().await.image_task.push_back(image_task);
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        Agent::create_data_channel(agent.clone()).await;
        let for_performance = agent.clone();
        let for_task_management = agent;
        tokio::spawn(async move {
            Agent::update_performance(for_performance).await;
        });
        tokio::spawn(async move {
            Agent::task_management(for_task_management).await;
        });
    }

    pub async fn terminate(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Terminating agent.".to_string()).await;
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
        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Termination complete.".to_string()).await;
    }

    async fn update_performance(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while !agent.read().await.terminate {
            if timer.elapsed() > timeout_duration {
                Logger::append_agent_log(uuid, LogLevel::WARNING, "Agent: Control Channel timeout.".to_string()).await;
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
                                Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to serialized confirm data.".to_string()).await;
                                continue;
                            }
                        } else {
                            Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse performance.".to_string()).await;
                            continue;
                        }
                    } else {
                        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
                        return;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
    }

    async fn task_management(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        while !agent.read().await.terminate {
            if let Some(mut image_task) = agent.write().await.image_task.pop_front() {
                if let Err(err) = Agent::transfer_task(agent.clone(), &image_task).await {
                    Logger::append_agent_log(uuid, LogLevel::ERROR, err).await;
                    TaskManager::submit_image_task(image_task, false).await;
                    continue;
                }
                let mut success = false;
                let mut data_channel_available = true;
                let mut timeout_timer = Instant::now();
                let timeout_duration = Duration::from_secs(config.control_channel_timeout);
                let mut polling_times = 0_u32;
                let polling_timer = Instant::now();
                let polling_interval = Duration::from_millis(config.polling_interval);
                while !agent.read().await.terminate {
                    if timeout_timer.elapsed() > timeout_duration {
                        Logger::append_agent_log(uuid, LogLevel::WARNING, "Agent: Data Channel timeout.".to_string()).await;
                        TaskManager::submit_image_task(image_task, false).await;
                        Agent::terminate(agent.clone()).await;
                        return;
                    }
                    if polling_timer.elapsed() > polling_interval * polling_times {
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
                                    Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
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
                                            Err(err) => Logger::append_agent_log(uuid, LogLevel::ERROR, format!("Agent: An error occurred while processing.\nReason: {err}")).await,
                                        }
                                    } else {
                                        Logger::append_agent_log(uuid, LogLevel::ERROR, "Agent: Unable to parse task result.".to_string()).await;
                                    }
                                    break;
                                } else {
                                    Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
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
                if !data_channel_available {
                    Agent::create_data_channel(agent.clone()).await;
                }
                TaskManager::submit_image_task(image_task, success).await;
            } else {
                if let Some(image_task) = Agent::steal_task(agent.clone()).await {
                    Agent::add_task(agent.clone(), image_task).await
                } else {
                    {
                        let mut agent = agent.write().await;
                        agent.idle_unused = Performance::calc_residual_usage(&agent.information, &agent.realtime_usage);
                    }
                    let mut data_channel_available = true;
                    let timer = Instant::now();
                    let mut timeout_timer = Instant::now();
                    let timeout_duration = Duration::from_secs(config.control_channel_timeout);
                    let idle_duration = Duration::from_secs(config.agent_idle_duration);
                    let mut polling_times = 0_u32;
                    let polling_interval = Duration::from_millis(config.polling_interval);
                    while !agent.read().await.terminate && timer.elapsed() <= idle_duration {
                        if timeout_timer.elapsed() > timeout_duration {
                            Logger::append_agent_log(uuid, LogLevel::WARNING, "Agent: Data Channel timeout.".to_string()).await;
                            Agent::terminate(agent.clone()).await;
                            return;
                        }
                        if timer.elapsed() > polling_interval * polling_times {
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
                                        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Channel has been closed.".to_string()).await;
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
                    if !data_channel_available {
                        Agent::create_data_channel(agent.clone()).await;
                    }
                }
            }
        }
    }

    async fn create_data_channel(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let (listener, port) = loop {
            if agent.read().await.terminate {
                return;
            }
            let port = match PortPool::allocate_port().await {
                Some(port) => port,
                None => {
                    Logger::append_agent_log(uuid, LogLevel::WARNING, "Agent: No available port for Data Channel".to_string()).await;
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            };
            match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => break (listener, port),
                Err(err) => {
                    PortPool::free_port(port).await;
                    Logger::append_system_log(LogLevel::ERROR, format!("Agent: Port binding failed.\nReason: {err}")).await;
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            }
        };
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let (tcp_stream, _) = loop {
            if agent.read().await.terminate || timer.elapsed() > timeout_duration {
                PortPool::free_port(port).await;
                return;
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
                        Err(_) => {
                            agent.write().await.control_channel_sender.send(DataChannelPortPacket::new(port)).await;
                            continue;
                        },
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
        Logger::append_agent_log(uuid, LogLevel::INFO, "Agent: Create Data channel successfully.".to_string()).await;
    }

    async fn transfer_task(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), String> {
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
            Agent::create_data_channel(agent).await;
            Err("Agent: Data Channel is not available.".to_string())?
        }
        Ok(())
    }

    async fn transfer_task_info(agent: Arc<RwLock<Agent>>, image_task: &ImageTask) -> Result<(), String> {
        let config = Config::now().await;
        let task_info = TaskInfo::new(image_task.task_uuid.clone(), image_task.model_filename.clone(), image_task.inference_type);
        let task_info_data = serde_json::to_vec(&task_info)
            .map_err(|_| "Agent: Unable to serialized task info data.".to_string())?;
        let time = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while !agent.read().await.terminate && time.elapsed() < timeout_duration {
            if time.elapsed() > polling_interval * polling_times {
                if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
                    data_channel_sender.send(TaskInfoPacket::new(task_info_data.clone())).await;
                } else {
                    Err("Agent: Data Channel is not available.".to_string())?
                }
                polling_times += 1;
            }
            if let Some(data_channel_receiver) = &mut agent.write().await.data_channel_receiver {
                select! {
                    reply = data_channel_receiver.task_info_reply_packet.recv() => {
                        reply.ok_or("Agent: Channel has been closed.".to_string())?;
                        clear_unbounded_channel(&mut data_channel_receiver.task_info_reply_packet).await;
                        return Ok(())
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err("Agent: Data Channel is not available.".to_string())?
            }
        }
        Err("Agent: Task Info retransmission limit reached.".to_string())
    }

    #[allow(unused_assignments)]
    async fn transfer_file(agent: Arc<RwLock<Agent>>, filename: &String, filepath: &PathBuf) -> Result<(), String> {
        let config = Config::now().await;
        let filesize = fs::metadata(&filepath).await
            .map_err(|err| format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display()))?
            .len();
        match &mut agent.write().await.data_channel_sender {
            Some(data_channel_sender) => {
                let file_header = serde_json::to_vec(&FileHeader::new(filename.clone(), filesize as usize))
                    .map_err(|_| "Agent: Unable to serialized file header data.".to_string())?;
                data_channel_sender.send(FileHeaderPacket::new(file_header)).await;
            },
            None => Err("Agent: Data channel is not available.".to_string())?,
        }
        let file = File::open(filepath.clone()).await;
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut sent_packets = Vec::new();
        let mut file = file
            .map_err(|err| format!("Agent: Cannot read file {filepath}.\nReason: {err}", filepath = filepath.display()))?;
        loop {
            let bytes_read = file.read(&mut buffer).await
                .map_err(|_| format!("Agent: An error occurred while reading file {filepath}.", filepath = filepath.display()))?;
            if bytes_read == 0 {
                break;
            }
            let mut data = sequence_number.to_be_bytes().to_vec();
            data.extend_from_slice(&buffer[..bytes_read]);
            match &mut agent.write().await.data_channel_sender {
                Some(data_channel_sender) => data_channel_sender.send(FileBodyPacket::new(data.clone())).await,
                None => Err("Agent: Data channel is not available.".to_string())?,
            }
            sent_packets.push(data);
            sequence_number += 1;
        }
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        let mut require_resend = Vec::new();
        while time.elapsed() < timeout_duration {
            match &mut agent.write().await.data_channel_receiver {
                Some(data_channel_receiver) => {
                    select! {
                        biased;
                        reply = data_channel_receiver.file_transfer_reply_packet.recv() => {
                            match &reply {
                                Some(reply_packet) => {
                                    clear_unbounded_channel(&mut data_channel_receiver.file_transfer_reply_packet).await;
                                    match serde_json::from_slice::<FileTransferResult>(reply_packet.as_data_byte()) {
                                        Ok(file_transfer_result) => {
                                            match file_transfer_result.into() {
                                                Some(missing_chunks) => require_resend = missing_chunks,
                                                None => return Ok(()),
                                            }
                                        },
                                        Err(_) => Err("Agent: Unable to parse file transfer result.")?,
                                    }
                                },
                                None => Err("Agent: Channel has been closed.".to_string())?,
                            }
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                }
                None => Err("Agent: Data channel is not available.".to_string())?,
            }
            for missing_chunk in &require_resend {
                if let Some(data) = sent_packets.get(*missing_chunk) {
                    match &mut agent.write().await.data_channel_sender {
                        Some(data_channel_sender) => data_channel_sender.send(FileBodyPacket::new(data.clone())).await,
                        None => Err("Agent: Data channel is not available.".to_string())?,
                    }
                }
            }
        }
        Err("Agent: File transfer timeout.".to_string())
    }

    pub async fn steal_task(agent: Arc<RwLock<Agent>>) -> Option<ImageTask> {
        let agents = AgentManager::sorted_by_vram().await;
        let (vram, ram) = {
            let agent = agent.write().await;
            (agent.idle_unused.vram, agent.idle_unused.ram)
        };
        for (uuid, _) in agents {
            if let Some(agent) = AgentManager::get_agent(uuid).await {
                let mut steal = false;
                let mut cache = false;
                let mut agent = agent.write().await;
                match agent.image_task.get(0) {
                    Some(image_task) => {
                        let estimate_vram = TaskManager::estimated_vram_usage(&image_task.model_filepath).await;
                        let estimate_ram = TaskManager::estimated_ram_usage(&image_task.image_filepath).await;
                        if vram > estimate_vram && ram > estimate_ram * 0.7 {
                            steal = true;
                            if ram < estimate_ram {
                                cache = true;
                            }
                        }
                    },
                    None => continue,
                }
                if steal {
                    match agent.image_task.pop_front() {
                        Some(mut image_task) => {
                            image_task.cache = cache;
                            return Some(image_task);
                        },
                        None => continue,
                    }
                }
            }
        }
        None
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

    pub fn mut_idle_unused(&mut self) -> &mut Performance {
        &mut self.idle_unused
    }

    pub fn realtime_usage(&self) -> &Performance {
        &self.realtime_usage
    }

    pub fn mut_realtime_usage(&mut self) -> &mut Performance {
        &mut self.realtime_usage
    }
}
