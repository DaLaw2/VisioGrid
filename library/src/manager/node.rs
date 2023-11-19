use std::path::PathBuf;
use tokio::time::{self, sleep, Duration, Instant};
use tokio::net::TcpListener;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use tokio::{fs, select};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use crate::utils::port_pool::PortPool;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::utils::performance::Performance;
use crate::manager::utils::image_resource::ImageResource;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::data_packet_channel;
use crate::connection::connection_channel::control_packet_channel;
use crate::connection::connection_channel::data_channel::DataChannel;
use crate::connection::connection_channel::control_channel::ControlChannel;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::manager::utils::task_info::TaskInfo;
use crate::utils::config::Config;
use crate::connection::utils::file_transfer_result::FileTransferResult;
use crate::manager::file_manager::FileManager;
use crate::manager::task_manager::TaskManager;
use crate::manager::utils::task::Task;

pub struct Node {
    pub id: usize,
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
    pub fn new(id: usize, socket_stream: SocketStream) -> Self {
        let (control_channel, control_packet_channel) = ControlChannel::new(id, socket_stream);
        Self {
            id,
            idle_unused: Performance::new(0.0, 0.0, 0.0, 0.0),
            realtime_usage: Performance::new(0.0, 0.0, 0.0, 0.0),
            task: VecDeque::new(),
            last_task: None,
            control_channel,
            data_channel: None,
            control_packet_channel,
            data_packet_channel: None,
        }
    }

    pub async fn run() {

    }

    pub async fn add_task(node: Arc<RwLock<Node>>, task: ImageResource) {
        node.write().await.task.push_back(task);
    }

    async fn transfer_task(node: Arc<RwLock<Node>>) {
        let mut success = true;
        let mut task_complete = false;
        let task = node.write().await.task.pop_front();
        match task {
            Some(task) => {
                match &mut self.data_channel {
                    Some(data_channel) => {
                        let should_transfer_model = if let Some(last_task) = &self.last_task {
                            task.task_uuid != last_task.task_uuid
                        } else {
                            true
                        };
                        if should_transfer_model {
                            data_channel.send(TaskInfoPacket::new(TaskInfo::new(&task))).await;
                            if let Err(err) = self.transfer_file(task.model_filename, task.model_filepath).await {
                                Logger::append_node_log(self.id, LogLevel::ERROR, err).await;
                                success = false;
                            }
                        }
                        if let Err(err) = self.transfer_file(task.image_filename, task.image_filepath).await {
                            Logger::append_node_log(self.id, LogLevel::ERROR, err).await;
                            success = false;
                        };
                    },
                    None => {
                        self.create_data_channel().await;
                        return;
                    },
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
                        None => Logger::append_node_log(self.id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)),
                    }
                }
                if task_complete {
                    match TaskManager::remove_task(task.task_uuid).await {
                        Some(task) => FileManager::add_postprocess_task(task).await,
                        None => Logger::append_node_log(self.id, LogLevel::ERROR, format!("Node: Task {} does not exist.", task.task_uuid)),
                    }
                }
            },
            None => {
                //如果沒有任務
                //任務竊取
                //如果竊取不到任務
                //休息
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
        let file = File::open(filepath.as_ref()).await;
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
                    _ = time::sleep(Duration::from_millis(config.internal_timestamp as u64)) => continue,
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
                connection = listener.accept().await => {
                    match connection {
                        Ok(connection) => break connection,
                        Err(_) => {
                            node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
                            continue;
                        }
                    }
                },
                _ = time::sleep(Duration::from_secs(config.data_channel_timout as u64)) => {
                    node.write().await.control_channel.send(DataChannelPortPacket::new(port)).await;
                    continue;
                },
            }
        };
        let socket_stream =  SocketStream::new(stream, address);
        let node = node.write().await;
        let (data_channel, data_packet_channel) = DataChannel::new(node.id, socket_stream);
        node.data_channel = Some(data_channel);
        node.data_packet_channel = Some(data_packet_channel);
        Logger::append_node_log(node.id, LogLevel::INFO, "Node: Create data channel successfully.".to_string()).await;
    }
}
