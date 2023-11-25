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
use crate::connection::utils::performance::Performance;
use crate::manager::utils::image_resource::ImageResource;
use crate::connection::socket::socket_stream::SocketStream;
use crate::manager::utils::node_information::NodeInformation;
use crate::connection::connection_channel::data_packet_channel;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::connection::connection_channel::control_packet_channel;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::utils::file_transfer_result::FileTransferResult;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;

pub struct Node {
    pub id: usize,
    pub node_information: NodeInformation,
    pub idle_unused: Performance,
    pub realtime_usage: Performance,
    task: VecDeque<ImageResource>,
    last_task: Option<ImageResource>,
    control_channel: ControlChannel,
    data_channel: Option<DataChannel>,
    control_packet_channel: control_packet_channel::PacketReceiver,
    data_packet_channel: Option<data_packet_channel::PacketReceiver>,
}

impl Node {
    pub async fn new(id: usize, socket_stream: SocketStream) -> Option<Self> {
        let config = Config::now().await;
        let mut start_time = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timout as u64);
        let (control_channel, mut control_packet_channel) = ControlChannel::new(id, socket_stream);
        while start_time.elapsed() < timeout_duration {
            select! {
                biased;
                reply = control_packet_channel.node_information_packet.recv() => {
                    match reply {
                        Some(packet) => {
                            match NodeInformation::from_str(&packet.data_to_string()) {
                               Ok(node_information) => return Some(Self {
                                    id,
                                    node_information,
                                    idle_unused: Performance::new(0.0, 0.0, 0.0, 0.0),
                                    realtime_usage: Performance::new(0.0, 0.0, 0.0, 0.0),
                                    task: VecDeque::new(),
                                    last_task: None,
                                    control_channel,
                                    data_channel: None,
                                    control_packet_channel,
                                    data_packet_channel: None,
                                }),
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

    pub async fn run() {

    }

    pub async fn add_task(node: Arc<RwLock<Node>>, task: ImageResource) {
        node.write().await.task.push_back(task);
    }

    async fn transfer_task(node: Arc<RwLock<Node>>) {
        let config = Config::now().await;
        let mut success = true;
        let mut task_complete = false;
        let (node_id, task) = {
            let mut node = node.write().await;
            (node.id, node.task.pop_front())
        };
        match task {
            Some(task) => {
                if node.write().await.data_channel.is_some() {
                    let should_transfer_model = if let Some(last_task) = &node.read().await.last_task {
                        task.task_uuid != last_task.task_uuid
                    } else {
                        true
                    };
                    if should_transfer_model {
                        match &mut node.write().await.data_channel {
                            Some(data_channel) => data_channel.send(TaskInfoPacket::new(TaskInfo::new(&task))).await,
                            None => {
                                Node::create_data_channel(node.clone()).await;
                                return;
                            }
                        }
                        if let Err(err) = Self::transfer_file(node.clone(), task.model_filename, task.model_filepath).await {
                            Logger::append_node_log(node_id, LogLevel::ERROR, err).await;
                            success = false;
                        }
                    }
                    if let Err(err) = Self::transfer_file(node.clone(), task.image_filename, task.image_filepath).await {
                        Logger::append_node_log(node_id, LogLevel::ERROR, err).await;
                        success = false;
                    };
                } else {
                    Node::create_data_channel(node).await;
                    return;
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
                        None => Logger::append_node_log(node_id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)).await,
                    }
                }
                if task_complete {
                    match TaskManager::remove_task(task.task_uuid).await {
                        Some(task) => FileManager::add_postprocess_task(task).await,
                        None => Logger::append_node_log(node_id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)).await,
                    }
                }
            },
            None => {
                match Node::steal_task(node.clone()).await {
                    Some(task) => node.write().await.task.push_back(task),
                    None => sleep(Duration::from_millis(config.internal_timestamp as u64)).await,
                }
            }
        }
    }

    async fn transfer_file(node: Arc<RwLock<Node>>, filename: String, filepath: PathBuf) -> Result<(), String> {
        let config = Config::now().await;
        let filesize = match fs::metadata(&filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => return Err(format!("Node: Cannot read file {}.", filepath.display())),
        };
        if let Some(data_channel) = &mut node.write().await.data_channel {
            data_channel.send(FileHeaderPacket::new(filename.clone(), filesize as usize)).await;
        } else {
            return Err("Node: Data channel is not available.".to_string());
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
                            if let Some(data_channel) = &mut node.write().await.data_channel {
                                data_channel.send(FileBodyPacket::new(data.clone())).await;
                            } else {
                                return Err("Node: Data channel is not available.".to_string());
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
        let mut retry_times = 0_usize;
        let mut start_time = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timout as u64);
        let mut require_resend = Vec::new();
        while retry_times < config.file_transfer_retry_times {
            if start_time.elapsed() >= timeout_duration {
                retry_times += 1;
                start_time = Instant::now();
            }
            if let Some(data_packet_channel) = &mut node.write().await.data_packet_channel {
                select! {
                    biased;
                    reply = data_packet_channel.file_transfer_reply_packet.recv() => {
                        match &reply {
                            Some(reply_packet) => {
                                if let Some(missing_chunks) = FileTransferResult::parse_from_packet(reply_packet).result {
                                    require_resend = missing_chunks;
                                } else {
                                    return Ok(());
                                }
                            },
                            None => return Err("Node: An error occurred while receive packet.".to_string()),
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
                }
            } else {
                return Err("Node: Data channel is not available.".to_string());
            }
            for missing_chunk in require_resend {
                if let Some(data) = sent_packets.get(&missing_chunk) {
                    if let Some(data_channel) = &mut node.write().await.data_channel {
                        data_channel.send(FileBodyPacket::new(data.clone())).await;
                    } else {
                        return Err("Node: Data channel is not available.".to_string());
                    }
                }
            }
        }
        Err("Node: File retransmission limit reached.".to_string())
    }

    pub async fn steal_task(node: Arc<RwLock<Node>>) -> Option<ImageResource> {
        let vram = node.read().await.idle_unused.gram;
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
        node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
        let (stream, address) = loop {
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
                _ = sleep(Duration::from_secs(config.data_channel_timout as u64)) => {
                    node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
                    continue;
                },
            }
        };
        let socket_stream =  SocketStream::new(stream, address);
        let mut node = node.write().await;
        let (data_channel, data_packet_channel) = DataChannel::new(node.id, socket_stream);
        node.data_channel = Some(data_channel);
        node.data_packet_channel = Some(data_packet_channel);
        Logger::append_node_log(node.id, LogLevel::INFO, "Node: Create data channel successfully.".to_string()).await;
    }
}
