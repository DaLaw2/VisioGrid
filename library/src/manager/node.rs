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
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task_manager::TaskManager;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::task_info::TaskInfo;
use crate::manager::utils::image_task::ImageTask;
use crate::connection::packet::definition::Packet;
use crate::manager::utils::task_result::TaskResult;
use crate::manager::utils::performance::Performance;
use crate::manager::utils::confirm_type::ConfirmType;
use crate::connection::packet::alive_packet::AlivePacket;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::packet::confirm_packet::ConfirmPacket;
use crate::manager::utils::node_information::NodeInformation;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::manager::utils::file_transfer_result::FileTransferResult;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::packet::still_process_packet::StillProcessPacket;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::connection_channel::data_channel_sender::DataChannelSender;
use crate::connection::connection_channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::connection_channel::control_channel_sender::ControlChannelSender;
use crate::connection::connection_channel::control_channel_receiver::ControlChannelReceiver;

pub struct Node {
    uuid: Uuid,
    terminate: bool,
    information: NodeInformation,
    idle_unused: Performance,
    realtime_usage: Performance,
    image_task: VecDeque<ImageTask>,
    previous_task: Option<ImageTask>,
    control_channel_sender: ControlChannelSender,
    control_channel_receiver: ControlChannelReceiver,
    data_channel_sender: Option<DataChannelSender>,
    data_channel_receiver: Option<DataChannelReceiver>,
}

impl Node {
    pub async fn new(uuid: Uuid, socket_stream: SocketStream) -> Option<Self> {
        let config = Config::now().await;
        let mut node_information: Option<NodeInformation> = None;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(uuid, socket_stream);
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            select! {
                biased;
                reply = control_channel_receiver.node_information_packet.recv() => {
                    return match &reply {
                        Some(packet) => {
                            match serde_json::from_slice::<NodeInformation>(&packet.as_data_byte()) {
                                Ok(information) => {
                                    node_information = Some(information);
                                    control_channel_sender.send(ConfirmPacket::new(ConfirmType::ReceiveNodeInformationSuccess)).await;
                                    continue;
                                },
                                Err(_) => {
                                    Logger::append_node_log(uuid, LogLevel::ERROR, "Node: Unable to parse information.".to_string()).await;
                                    None
                                },
                            }
                        },
                        None => {
                            Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                            None
                        },
                    };
                },
                reply = control_channel_receiver.performance_packet.recv() => {
                    return match &reply {
                        Some(packet) => {
                            match serde_json::from_slice::<Performance>(packet.as_data_byte()) {
                                Ok(realtime_usage) => {
                                    match node_information {
                                        Some(information) => {
                                            control_channel_sender.send(ConfirmPacket::new(ConfirmType::ReceivePerformanceSuccess)).await;
                                            let residual_usage = Performance::calc_residual_usage(&information, &realtime_usage);
                                            let node = Self {
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
                                            Some(node)
                                        },
                                        None => {
                                            Logger::append_node_log(uuid, LogLevel::ERROR, "Node: Node information not ready.".to_string()).await;
                                            None
                                        }
                                    }
                                },
                                Err(_) => {
                                    Logger::append_node_log(uuid, LogLevel::ERROR, "Node: Unable to parse performance.".to_string()).await;
                                    None
                                },
                            }
                        },
                        None => {
                            Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                            None
                        },
                    };
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        None
    }

    pub async fn add_task(node: Arc<RwLock<Node>>, image_task: ImageTask) {
        node.write().await.image_task.push_front(image_task);
    }

    pub async fn run(node: Arc<RwLock<Node>>) {
        Node::create_data_channel(node.clone()).await;
        let for_performance = node.clone();
        let for_task_management = node;
        tokio::spawn(async move {
            Node::update_performance(for_performance).await;
        });
        tokio::spawn(async move {
            Node::task_management(for_task_management).await;
        });
    }

    pub async fn terminate(node: Arc<RwLock<Node>>) {
        let uuid = node.read().await.uuid;
        Logger::append_node_log(uuid, LogLevel::INFO, "Node: Terminating node.".to_string()).await;
        let image_task = {
            let mut node = node.write().await;
            node.terminate = true;
            node.control_channel_sender.disconnect().await;
            node.control_channel_receiver.disconnect().await;
            if let Some(data_channel_sender) = &mut node.data_channel_sender {
                data_channel_sender.disconnect().await;
            }
            if let Some(data_channel_receiver) = &mut node.data_channel_receiver {
                data_channel_receiver.disconnect().await;
            }
            mem::take(&mut node.image_task)
        };
        TaskManager::redistribute_task(image_task).await;
        NodeCluster::remove_node(uuid).await;
    }

    async fn update_performance(node: Arc<RwLock<Node>>) {
        let uuid = node.read().await.uuid;
        let config = Config::now().await;
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while !node.read().await.terminate {
            if timer.elapsed() > timeout_duration {
                Logger::append_node_log(uuid, LogLevel::WARNING, "Node: Control Channel timeout.".to_string()).await;
                Node::terminate(node).await;
                return;
            }
            let mut node = node.write().await;
            select! {
                biased;
                reply = node.control_channel_receiver.performance_packet.recv() => {
                    match &reply {
                        Some(reply_packet) => {
                            match serde_json::from_slice::<Performance>(reply_packet.as_data_byte()) {
                                Ok(performance) => {
                                    node.realtime_usage = performance;
                                    node.control_channel_sender.send(ConfirmPacket::new(ConfirmType::ReceivePerformanceSuccess)).await;
                                    timer = Instant::now();
                                },
                                Err(_) => {
                                    Logger::append_node_log(uuid, LogLevel::ERROR, "Node: Unable to parse performance.".to_string()).await;
                                    continue;
                                },
                            }
                        },
                        None => {
                            Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                            return;
                        },
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
    }

    async fn task_management(node: Arc<RwLock<Node>>) {
        let uuid = node.read().await.uuid;
        let config = Config::now().await;
        while !node.read().await.terminate {
            match node.write().await.image_task.pop_back() {
                Some(mut image_task) => {
                    if let Err(err) = Node::transfer_task(node.clone(), &image_task).await {
                        Logger::append_node_log(uuid, LogLevel::ERROR, err).await;
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
                    loop {
                        if timeout_timer.elapsed() > timeout_duration {
                            Logger::append_node_log(uuid, LogLevel::WARNING, "Node: Data Channel timeout.".to_string()).await;
                            TaskManager::submit_image_task(image_task, false).await;
                            Node::terminate(node.clone()).await;
                            return;
                        }
                        if polling_timer.elapsed() > polling_interval * polling_times {
                            match &mut node.write().await.data_channel_sender {
                                Some(data_channel_sender) => data_channel_sender.send(StillProcessPacket::new()).await,
                                None => {
                                    data_channel_available = false;
                                    break;
                                },
                            }
                            polling_times += 1;
                        }
                        match &mut node.write().await.data_channel_receiver {
                            Some(data_channel_receiver) => {
                                select! {
                                    biased;
                                    reply = data_channel_receiver.still_process_reply_packet.recv() => {
                                        match &reply {
                                            Some(_) => timeout_timer = Instant::now(),
                                            None => {
                                                Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                                                return;
                                            },
                                        }
                                    },
                                    reply = data_channel_receiver.result_packet.recv() => {
                                        match &reply {
                                            Some(reply_packet) => {
                                                if let Ok(task_result) = serde_json::from_slice::<TaskResult>(reply_packet.as_data_byte()) {
                                                    if let Ok(bounding_box) = task_result.into() {
                                                        image_task.bounding_boxes = bounding_box;
                                                        success = true;
                                                    }
                                                }
                                                break;
                                            },
                                            None => {
                                                Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                                                return;
                                            },
                                        }
                                    },
                                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                                }
                            },
                            None => {
                                data_channel_available = false;
                                break;
                            },
                        }
                    }
                    if !data_channel_available {
                        Node::create_data_channel(node.clone()).await;
                        success = false;
                    }
                    TaskManager::submit_image_task(image_task, success).await;
                },
                None => {
                    match Node::steal_task(node.clone()).await {
                        Some(image_task) => Node::add_task(node.clone(), image_task).await,
                        None => {
                            {
                                let mut node = node.write().await;
                                node.idle_unused = Performance::calc_residual_usage(&node.information, &node.realtime_usage);
                            }
                            let mut data_channel_available = true;
                            let timer = Instant::now();
                            let mut timeout_timer = Instant::now();
                            let timeout_duration = Duration::from_secs(config.control_channel_timeout);
                            let idle_duration = Duration::from_secs(config.node_idle_duration);
                            let mut polling_times = 0_u32;
                            let polling_interval = Duration::from_millis(config.polling_interval);
                            while timer.elapsed() <= idle_duration {
                                if timeout_timer.elapsed() > timeout_duration {
                                    Logger::append_node_log(uuid, LogLevel::WARNING, "Node: Data Channel timeout.".to_string()).await;
                                    Node::terminate(node.clone()).await;
                                    return;
                                }
                                if timer.elapsed() > polling_interval * polling_times {
                                    match &mut node.write().await.data_channel_sender {
                                        Some(data_channel_sender) => data_channel_sender.send(AlivePacket::new()).await,
                                        None => {
                                            data_channel_available = false;
                                            break;
                                        },
                                    }
                                    polling_times += 1;
                                }
                                match &mut node.write().await.data_channel_receiver {
                                    Some(data_channel_receiver) => {
                                        select! {
                                            biased;
                                            reply = data_channel_receiver.alive_reply_packet.recv() => {
                                                match &reply {
                                                    Some(_) => timeout_timer = Instant::now(),
                                                    None => {
                                                        Logger::append_node_log(uuid, LogLevel::INFO, "Node: Channel has been closed.".to_string()).await;
                                                        return;
                                                    },
                                                }
                                            },
                                            _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                                        }
                                    }
                                    None => {
                                        data_channel_available = false;
                                        break;
                                    },
                                }
                            }
                            if !data_channel_available {
                                Node::create_data_channel(node.clone()).await;
                            }
                        }
                    }
                }
            }
        }
    }

    async fn create_data_channel(node: Arc<RwLock<Node>>) {
        let uuid = node.read().await.uuid;
        let config = Config::now().await;
        let (listener, port) = loop {
            let port = match PortPool::allocate_port().await {
                Some(port) => port,
                None => {
                    Logger::append_node_log(uuid, LogLevel::WARNING, "Node: No available port for Data Channel".to_string()).await;
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            };
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(listener) => break (listener, port),
                Err(err) => {
                    PortPool::free_port(port).await;
                    Logger::append_system_log(LogLevel::ERROR, format!("Node: Port binding failed.\nReason: {}\n", err)).await;
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            }
        };
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let (stream, address) = loop {
            if timer.elapsed() > timeout_duration {
                PortPool::free_port(port).await;
                return;
            }
            if timer.elapsed() > polling_times * polling_interval {
                node.write().await.control_channel_sender.send(DataChannelPortPacket::new(port)).await;
                polling_times += 1;
            }
            select! {
                biased;
                connection = listener.accept() => {
                    match connection {
                        Ok(connection) => break connection,
                        Err(_) => {
                            node.write().await.control_channel_sender.send(DataChannelPortPacket::new(port)).await;
                            continue;
                        },
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        };
        let socket_stream = SocketStream::new(stream, address);
        let (data_channel_sender, data_channel_receiver) = DataChannel::new(uuid, socket_stream);
        let mut node = node.write().await;
        node.data_channel_sender = Some(data_channel_sender);
        node.data_channel_receiver = Some(data_channel_receiver);
        Logger::append_node_log(uuid, LogLevel::INFO, "Node: Create Data channel successfully.".to_string()).await;
    }

    async fn transfer_task(node: Arc<RwLock<Node>>, image_task: &ImageTask) -> Result<(), String> {
        if node.write().await.data_channel_sender.is_some() {
            let should_transfer_model = if let Some(last_task) = &node.read().await.previous_task {
                image_task.task_uuid != last_task.task_uuid
            } else {
                true
            };
            Node::transfer_task_info(node.clone(), &image_task).await?;
            if should_transfer_model {
                Node::transfer_file(node.clone(), &image_task.model_filename, &image_task.model_filepath).await?;
            }
            Node::transfer_file(node.clone(), &image_task.image_filename, &image_task.image_filepath).await?;
        } else {
            Node::create_data_channel(node).await;
            Err("Node: Data Channel is not available.".to_string())?
        }
        Ok(())
    }

    async fn transfer_task_info(node: Arc<RwLock<Node>>, image_task: &ImageTask) -> Result<(), String> {
        let config = Config::now().await;
        let time = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while time.elapsed() < timeout_duration {
            if time.elapsed() > polling_interval * polling_times {
                match &mut node.write().await.data_channel_sender {
                    Some(data_channel_sender) => data_channel_sender.send(TaskInfoPacket::new(TaskInfo::new(&image_task))).await,
                    None => Err("Node: Data Channel is not available.".to_string())?,
                }
                polling_times += 1;
            }
            match &mut node.write().await.data_channel_receiver {
                Some(data_channel_receiver) => {
                    select! {
                        reply = data_channel_receiver.task_info_reply_packet.recv() => {
                            return match &reply {
                                Some(_) => Ok(()),
                                None => Err("Node: Channel has been closed.".to_string()),
                            }
                        }
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                }
                None => Err("Node: Data Channel is not available.".to_string())?,
            }
        }
        Err("Node: Task Info retransmission limit reached.".to_string())
    }

    #[allow(unused_assignments)]
    async fn transfer_file(node: Arc<RwLock<Node>>, filename: &String, filepath: &PathBuf) -> Result<(), String> {
        let config = Config::now().await;
        let filesize = match fs::metadata(&filepath).await {
            Ok(metadata) => metadata.len(),
            Err(err) => Err(format!("Node: Cannot read file {}.\nReason: {}", filepath.display(), err))?,
        };
        match &mut node.write().await.data_channel_sender {
            Some(data_channel_sender) => data_channel_sender.send(FileHeaderPacket::new(filename.clone(), filesize as usize)).await,
            None => Err("Node: Data channel is not available.".to_string())?,
        }
        let file = File::open(filepath.clone()).await;
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut sent_packets = Vec::new();
        match file {
            Ok(mut file) => {
                loop {
                    let bytes_read = file.read(&mut buffer).await;
                    match bytes_read {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                break;
                            }
                            let mut data = sequence_number.to_be_bytes().to_vec();
                            data.extend_from_slice(&buffer[..bytes_read]);
                            match &mut node.write().await.data_channel_sender {
                                Some(data_channel_sender) => data_channel_sender.send(FileBodyPacket::new(data.clone())).await,
                                None => Err("Node: Data channel is not available.".to_string())?,
                            }
                            sent_packets.push(data);
                            sequence_number += 1;
                        },
                        Err(_) => Err(format!("Node: An error occurred while reading file {}.", filepath.display()))?,
                    }
                }
            },
            Err(err) => Err(format!("Node: Cannot read file {}.\nReason: {}", filepath.display(), err))?,
        }
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        let mut require_resend = Vec::new();
        while time.elapsed() < timeout_duration {
            match &mut node.write().await.data_channel_receiver {
                Some(data_channel_receiver) => {
                    select! {
                        biased;
                        reply = data_channel_receiver.file_transfer_reply_packet.recv() => {
                            match &reply {
                                Some(reply_packet) => {
                                    match FileTransferResult::parse_from_packet(reply_packet).into() {
                                        Some(missing_chunks) => require_resend = missing_chunks,
                                        None => return Ok(()),
                                    }
                                },
                                None => Err("Node: Channel has been closed.".to_string())?,
                            }
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                }
                None => Err("Node: Data channel is not available.".to_string())?,
            }
            for missing_chunk in &require_resend {
                if let Some(data) = sent_packets.get(*missing_chunk) {
                    match &mut node.write().await.data_channel_sender {
                        Some(data_channel_sender) => data_channel_sender.send(FileBodyPacket::new(data.clone())).await,
                        None => Err("Node: Data channel is not available.".to_string())?,
                    }
                }
            }
        }
        Err("Node: File transfer timeout.".to_string())
    }

    pub async fn steal_task(node: Arc<RwLock<Node>>) -> Option<ImageTask> {
        let nodes = NodeCluster::sorted_by_vram().await;
        let (vram, ram) = {
            let node = node.write().await;
            (node.idle_unused.vram, node.idle_unused.ram)
        };
        for (uuid, _) in nodes {
            if let Some(node) = NodeCluster::get_node(uuid).await {
                let mut steal = false;
                let mut cache = false;
                let mut node = node.write().await;
                match node.image_task.get(1) {
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
                    match node.image_task.remove(1) {
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

    pub fn node_information(&self) -> &NodeInformation {
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
