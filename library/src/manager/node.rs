use std::sync::Arc;
use tokio::fs::File;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::{fs, select};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use std::collections::{HashMap, VecDeque};
use tokio::time::{sleep, Duration, Instant};
use crate::utils::config::Config;
use crate::utils::port_pool::PortPool;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::file_manager::FileManager;
use crate::manager::task_manager::TaskManager;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::task_info::TaskInfo;
use crate::connection::packet::definition::Packet;
use crate::manager::utils::performance::Performance;
use crate::manager::utils::image_task::ImageTask;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::packet::confirm_packet::ConfirmPacket;
use crate::manager::utils::node_information::NodeInformation;
use crate::connection::connection_channel::data_packet_channel;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::connection::connection_channel::control_packet_channel;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::manager::utils::file_transfer_result::FileTransferResult;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::alive_packet::AlivePacket;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::packet::still_process_packet::StillProcessPacket;
use crate::manager::utils::task_result::TaskResult;

pub struct Node {
    id: usize,
    information: NodeInformation,
    terminate: bool,
    idle_unused: Performance,
    realtime_usage: Performance,
    task: VecDeque<ImageTask>,
    last_task: Option<ImageTask>,
    control_channel: ControlChannel,
    data_channel: Option<DataChannel>,
    control_packet_channel: control_packet_channel::PacketReceiver,
    data_packet_channel: Option<data_packet_channel::PacketReceiver>,
}

impl Node {
    pub async fn new(id: usize, socket_stream: SocketStream) -> Option<Self> {
        let config = Config::now().await;
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
        let (mut control_channel, mut control_packet_channel) = ControlChannel::new(id, socket_stream);
        while time.elapsed() < timeout_duration {
            select! {
                biased;
                reply = control_packet_channel.node_information_packet.recv() => {
                    match &reply {
                        Some(packet) => {
                            match serde_json::from_slice::<NodeInformation>(&packet.as_data_byte()) {
                                Ok(information) => {
                                    control_channel.send(ConfirmPacket::new()).await;
                                    let node = Self {
                                        id,
                                        information,
                                        terminate: false,
                                        idle_unused: Performance::default(),
                                        realtime_usage: Performance::default(),
                                        task: VecDeque::new(),
                                        last_task: None,
                                        control_channel,
                                        data_channel: None,
                                        control_packet_channel,
                                        data_packet_channel: None,
                                    };
                                    return Some(node);
                                },
                                Err(_) => return None,
                            }
                        },
                        None => return None,
                    }
                },
                _ = sleep(Duration::from_secs(config.internal_timestamp as u64)) => continue,
            }
        }
        None
    }

    pub async fn add_task(node: Arc<RwLock<Node>>, task: ImageTask) {
        node.write().await.task.push_back(task);
    }

    pub async fn run(node: Arc<RwLock<Node>>) {
        Node::create_data_channel(node.clone()).await;
        let node_for_performance = node.clone();
        tokio::spawn(async move {
            Node::update_performance(node_for_performance).await;
        });
        let node_for_task_management = node;
        tokio::spawn(async move {
            Node::task_management(node_for_task_management).await;
        });
    }

    pub async fn terminate(node: Arc<RwLock<Node>>) {
        let node_id = node.read().await.id;
        node.write().await.terminate = true;
        NodeCluster::remove_node(node_id).await;
        Logger::append_node_log(node_id, LogLevel::INFO, "Node: Terminating node.".to_string()).await;
        unimplemented!("重新分配任務");
    }

    async fn update_performance(node: Arc<RwLock<Node>>) {
        let node_id = node.read().await.id;
        let config = Config::now().await;
        let mut time = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
        loop {
            if node.read().await.terminate {
                return;
            }
            if time.elapsed() > timeout_duration {
                Logger::append_node_log(node_id, LogLevel::WARNING, "Node: Control Channel timout.".to_string()).await;
                Node::terminate(node).await;
                return;
            }
            let mut node = node.write().await;
            select! {
                biased;
                reply = node.control_packet_channel.performance_packet.recv() => {
                    match &reply {
                        Some(reply_packet) => {
                            match serde_json::from_slice::<Performance>(reply_packet.as_data_byte()) {
                                Ok(performance) => {
                                    node.realtime_usage = performance;
                                    node.control_channel.send(ConfirmPacket::new()).await;
                                    time = Instant::now();
                                },
                                Err(_) => continue,
                            }
                        },
                        None => continue,
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
            }
        }
    }

    async fn task_management(node: Arc<RwLock<Node>>) {
        unimplemented!("需要修改錯誤處理及最佳化代碼");
        let node_id = node.read().await.id;
        let config = Config::now().await;
        loop {
            if node.read().await.terminate {
                return;
            }
            let task = node.write().await.task.pop_front();
            match task {
                Some(task) => {
                    Node::transfer_task(node.clone(), task).await;
                    let mut success = false;
                    let mut data_channel = true;
                    let mut polling_times = 0_u32;
                    let polling_timer = Instant::now();
                    let mut timeout_timer = Instant::now();
                    let polling_interval = Duration::from_millis(config.polling_interval as u64);
                    let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
                    loop {
                        if timeout_timer.elapsed() > timeout_duration {
                            Logger::append_node_log(node_id, LogLevel::WARNING, "Node: Data Channel timout.".to_string()).await;
                            Node::terminate(node).await;
                            return;
                        }
                        if polling_timer.elapsed() > polling_interval * polling_times {
                            match &mut node.write().await.data_channel {
                                Some(data_channel) => data_channel.send(StillProcessPacket::new()).await,
                                None => {
                                    data_channel = false;
                                    break;
                                },
                            }
                            polling_times += 1;
                        }
                        match &mut node.write().await.data_packet_channel {
                            Some(data_packet_channel) => {
                                select! {
                                    biased;
                                    reply = data_packet_channel.result_packet.recv() => {
                                        match &reply {
                                            Some(reply_packet) => {
                                                if let Ok(task_result) = serde_json::from_slice::<TaskResult>(reply_packet.as_data_byte()) {
                                                    if let Ok(bounding_box) = task_result.result {
                                                        task.bounding_boxes = bounding_box;
                                                        success = true;
                                                    }
                                                }
                                                break;
                                            },
                                            None => break,
                                        }
                                    },
                                    reply = data_packet_channel.still_process_reply_packet.recv() => {
                                        match &reply {
                                            Some(reply_packet) => timeout_timer = Instant::now(),
                                            None => continue,
                                        }
                                    },
                                    _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
                                }
                            },
                            None => {
                                data_channel = false;
                                break;
                            },
                        }
                    }
                    if !data_channel {
                        Node::create_data_channel(node.clone()).await;
                    }
                    TaskManager::update_task_status(task.task_uuid.clone(), success).await;
                },
                None => {
                    match Node::steal_task(node.clone()).await {
                        Some(task) => Node::add_task(node.clone(), task).await;
                        None => {
                            let mut data_channel = true;
                            let mut polling_times = 0_u32;
                            let timer = Instant::now();
                            let mut timeout_timer = Instant::now();
                            let polling_interval = Duration::from_millis(config.polling_interval as u64);
                            let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
                            let idle_duration = Duration::from_secs(config.node_idle_duration as u64);
                            loop {
                                if timeout_timer.elapsed() > timeout_duration {
                                    Logger::append_node_log(node_id, LogLevel::WARNING, "Node: Data Channel timout.".to_string()).await;
                                    Node::terminate(node).await;
                                    return;
                                }
                                if timer.elapsed() > idle_duration {
                                    break;
                                }
                                if timer.elapsed() > polling_interval * polling_times {
                                    match &mut node.write().await.data_channel {
                                        Some(data_channel) => data_channel.send(AlivePacket::new()).await,
                                        None => {
                                            data_channel = false;
                                            break;
                                        }
                                    }
                                    polling_times += 1;
                                }
                                match &mut node.write().await.data_packet_channel {
                                    Some(data_packet_channel) => {
                                        select! {
                                            reply = data_packet_channel.alive_reply_packet.recv() => {
                                                match &reply {
                                                    Some(reply_packet) => timeout_timer = Instant::now(),
                                                    None => continue,
                                                }
                                            },
                                            _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
                                        }
                                    },
                                    None => {
                                        data_channel = false;
                                        break;
                                    },
                                }
                            }
                            if !data_channel {
                                Node::create_data_channel(node.clone()).await;
                            }
                        }
                    }
                }
            }
        }
    }

    async fn create_data_channel(node: Arc<RwLock<Node>>) {
        let config = Config::now().await;
        let (listener, port) = loop {
            let port = match PortPool::allocate_port().await {
                Some(port) => port,
                None => continue,
            };
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(listener) => break (listener, port),
                Err(_) => continue,
            }
        };
        let time = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval as u64);
        let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
        let (stream, address) = loop {
            if time.elapsed() > timeout_duration {
                return;
            }
            if time.elapsed() > polling_interval * polling_times {
                node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
                polling_times += 1;
            }
            select! {
                biased;
                connection = listener.accept() => {
                    match connection {
                        Ok(connection) => break connection,
                        Err(_) => {
                            node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
                            continue;
                        }
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
            }
        };
        let mut node = node.write().await;
        let socket_stream = SocketStream::new(stream, address);
        let (data_channel, data_packet_channel) = DataChannel::new(node.id, socket_stream);
        node.data_channel = Some(data_channel);
        node.data_packet_channel = Some(data_packet_channel);
        Logger::append_node_log(node.id, LogLevel::INFO, "Node: Create Data channel successfully.".to_string()).await;
    }

    async fn transfer_task(node: Arc<RwLock<Node>>, task: ImageTask) {
        let mut success = true;
        let mut task_complete = false;
        let node_id = node.read().await.id;
        if node.write().await.data_channel.is_some() {
            let should_transfer_model = if let Some(last_task) = &node.read().await.last_task {
                task.task_uuid != last_task.task_uuid
            } else {
                true
            };
            if should_transfer_model {
                if let Err(err) = Node::transfer_task_info(node.clone(), &task).await {
                    Logger::append_node_log(node_id, LogLevel::ERROR, err).await;
                    success = false;
                }
                if let Err(err) = Node::transfer_file(node.clone(), task.model_filename, task.model_filepath).await {
                    Logger::append_node_log(node_id, LogLevel::ERROR, err).await;
                    success = false;
                }
            }
            if let Err(err) = Node::transfer_file(node.clone(), task.image_filename, task.image_filepath).await {
                Logger::append_node_log(node_id, LogLevel::ERROR, err).await;
                success = false;
            };
        } else {
            Node::create_data_channel(node).await;
            success = false;
        }
        if !success {
            match TaskManager::instance_mut().await.get_task_mut(task.task_uuid).await {
                Some(task) => {
                    task.processed += 1;
                    task.unprocessed -= 1;
                    if task.unprocessed == 0 {
                        task_complete = true;
                    }
                }
                None => {
                    Logger::append_node_log(node_id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)).await;
                    return;
                },
            }
        }
        if task_complete {
            match TaskManager::remove_task(task.task_uuid).await {
                Some(task) => FileManager::add_postprocess_task(task).await,
                None => Logger::append_node_log(node_id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)).await,
            }
        }
    }

    async fn transfer_task_info(node: Arc<RwLock<Node>>, task: &ImageTask) -> Result<(), String> {
        let config = Config::now().await;
        let time = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval as u64);
        let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
        while time.elapsed() < timeout_duration {
            if time.elapsed() > polling_interval * polling_times {
                match &mut node.write().await.data_channel {
                    Some(data_channel) => data_channel.send(TaskInfoPacket::new(TaskInfo::new(&task))).await,
                    None => return Err("Node: Data Channel is not available.".to_string()),
                }
                polling_times += 1;
            }
            match &mut node.write().await.data_packet_channel {
                Some(data_packet_channel) => {
                    select! {
                        reply = data_packet_channel.task_info_reply_packet.recv() => {
                            match &reply {
                                Some(reply_packet) => return Ok(()),
                                None => return Err("Node: An error occurred while receive packet.".to_string()),
                            }
                        }
                        _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
                    }
                },
                None => return Err("Node: Data Channel is not available.".to_string()),
            }
        }
        Err("Node: Task Info retransmission limit reached.".to_string())
    }

    async fn transfer_file(node: Arc<RwLock<Node>>, filename: String, filepath: PathBuf) -> Result<(), String> {
        let config = Config::now().await;
        let filesize = match fs::metadata(&filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => return Err(format!("Node: Cannot read file {}.", filepath.display())),
        };
        match &mut node.write().await.data_channel {
            Some(data_channel) => data_channel.send(FileHeaderPacket::new(filename.clone(), filesize as usize)).await,
            None => return Err("Node: Data channel is not available.".to_string()),
        }
        let file = File::open(filepath.clone()).await;
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut sent_packets = HashMap::new();
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
                            match &mut node.write().await.data_channel {
                                Some(data_channel) => data_channel.send(FileBodyPacket::new(data.clone())).await,
                                None => return Err("Node: Data channel is not available.".to_string()),
                            }
                            sent_packets.insert(sequence_number, data);
                            sequence_number += 1;
                        },
                        Err(_) => return Err(format!("Node: An error occurred while reading {} file", filepath.display())),
                    }
                }
            },
            Err(_) => return Err(format!("Node: Cannot read file {}.", filepath.display())),
        }
        let time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timout as u64);
        let mut require_resend = Vec::new();
        while time.elapsed() < timeout_duration {
            match &mut node.write().await.data_packet_channel {
                Some(data_packet_channel) => {
                    select! {
                        biased;
                        reply = data_packet_channel.file_transfer_reply_packet.recv() => {
                            match &reply {
                                Some(reply_packet) => {
                                    match FileTransferResult::parse_from_packet(reply_packet).into() {
                                        Some(missing_chunks) => require_resend = missing_chunks,
                                        None => return Ok(()),
                                    }
                                },
                                None => return Err("Node: An error occurred while receive packet.".to_string()),
                            }
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
                    }
                },
                None => return Err("Node: Data channel is not available.".to_string()),
            }
            for missing_chunk in require_resend {
                if let Some(data) = sent_packets.get(&missing_chunk) {
                    match &mut node.write().await.data_channel {
                        Some(data_channel) => data_channel.send(FileBodyPacket::new(data.clone())).await,
                        None => return Err("Node: Data channel is not available.".to_string()),
                    }
                }
            }
        }
        Err("Node: File retransmission limit reached.".to_string())
    }

    pub async fn steal_task(node: Arc<RwLock<Node>>) -> Option<ImageTask> {
        let vram = node.read().await.idle_unused.vram;
        let filter_nodes = NodeCluster::filter_node_by_vram(vram).await;
        let mut task = None;
        for (node_id, _) in filter_nodes {
            if let Some(node) = NodeCluster::get_node(node_id).await {
                let mut node = node.write().await;
                if node.task.len() < 1 {
                    continue;
                } else {
                    task = node.task.pop_back();
                    break;
                }
            }
        }
        task
    }

    pub fn id(&self) -> usize {
        self.id
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
