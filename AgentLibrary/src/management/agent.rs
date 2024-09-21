use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::{ControlChannel, DataChannel};
use crate::connection::packet::agent_info_packet::AgentInfoPacket;
use crate::connection::packet::alive_ack_packet::AliveAckPacket;
use crate::connection::packet::control_ack_packet::ControlAckPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::connection::packet::file_header_ack_packet::FileHeaderAckPacket;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::packet::file_transfer_end_packet::FileTransferEndPacket;
use crate::connection::packet::file_transfer_result_packet::FileTransferResultPacket;
use crate::connection::packet::performance_packet::PerformancePacket;
use crate::connection::packet::task_result_packet::TaskResultPacket;
use crate::connection::packet::task_info_ack_packet::TaskInfoAckPacket;
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::SocketStream;
use crate::management::inference_manager::InferenceManager;
use crate::management::monitor::Monitor;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::file_transfer_result::FileTransferResult;
use crate::management::utils::inference_argument::ModelType;
use crate::management::utils::task_result::TaskResult;
use crate::management::utils::task_info::TaskInfo;
use crate::utils::clear_unbounded_channel;
use crate::utils::config::Config;
use crate::utils::logging::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::{sleep, Instant};
use uuid::Uuid;
use crate::connection::packet::still_process_ack_packet::StillProcessAckPacket;

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
        let mut information_confirm = false;
        let information = serde_json::to_vec(&Monitor::get_system_info().await)
            .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
        let (mut control_channel_sender, mut control_channel_receiver) = ControlChannel::new(socket_stream);
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while timer.elapsed() <= timeout_duration {
            if timer.elapsed() > polling_times * polling_interval {
                if !information_confirm {
                    control_channel_sender.send(AgentInfoPacket::new(information.clone())).await;
                } else {
                    let performance = Monitor::get_performance().await;
                    let performance_data = serde_json::to_vec(&performance)
                        .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
                    control_channel_sender.send(PerformancePacket::new(performance_data)).await;
                }
                polling_times += 1;
            }
            select! {
                biased;
                packet = control_channel_receiver.agent_info_ack_packet.recv() => {
                    let _ = packet.ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                    information_confirm = true;
                }
                packet = control_channel_receiver.performance_ack_packet.recv() => {
                    let _ = packet.ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                    if !information_confirm {
                        Err(error_entry!(MiscEntry::WrongDeliverOrder))?;
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
                }
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        Err(information_entry!(NetworkEntry::ControlChannelTimeout))
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

    pub async fn terminate(agent: &Arc<RwLock<Agent>>) {
        logging_information!(SystemEntry::Terminating);
        let mut agent = agent.write().await;
        agent.control_channel_sender.disconnect().await;
        agent.control_channel_receiver.disconnect().await;
        if let Some(data_channel_sender) = &mut agent.data_channel_sender {
            data_channel_sender.disconnect().await;
        }
        if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
            data_channel_receiver.disconnect().await;
        }
        logging_information!(SystemEntry::TerminateComplete);
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
                logging_information!(NetworkEntry::ControlChannelTimeout);
                break;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                let performance = Monitor::get_performance().await;
                match serde_json::to_vec(&performance) {
                    Ok(performance_data) => {
                        agent.write().await.control_channel_sender
                            .send(PerformancePacket::new(performance_data)).await;
                    }
                    Err(err) => logging_error!(IOEntry::SerdeSerializeError(err))
                }
                polling_times += 1;
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                reply = agent.control_channel_receiver.performance_ack_packet.recv() => {
                    if reply.is_some() {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_ack_packet).await;
                        timeout_timer = Instant::now();
                    } else {
                        logging_information!(NetworkEntry::ChannelClosed);
                        break;
                    }
                }
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        agent.write().await.state = AgentState::Terminate;
    }

    async fn management(agent: Arc<RwLock<Agent>>) {
        loop {
            Self::refresh_state(&agent).await;
            let state = agent.read().await.state;
            let result = match state {
                AgentState::ProcessTask => Self::process_task(&agent).await,
                AgentState::Idle(idle_time) => Self::idle(&agent, Duration::from_secs(idle_time)).await,
                AgentState::CreateDataChannel => Self::create_data_channel(&agent).await,
                AgentState::Terminate => {
                    Self::terminate(&agent).await;
                    return;
                }
                _ => Ok(())
            };
            if let Err(entry) = result {
                logging_entry!(entry);
            }
        }
    }

    async fn refresh_state(agent: &Arc<RwLock<Agent>>) {
        {
            let mut agent = agent.write().await;
            clear_unbounded_channel(&mut agent.control_channel_receiver.control_packet).await;
        }
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
                            match serde_json::from_slice::<AgentState>(packet.as_data_byte()) {
                                Ok(state) => agent.state = state,
                                Err(err) => {
                                    logging_error!(IOEntry::SerdeDeserializeError(err));
                                    continue;
                                }
                            }
                        }
                        None => {
                            logging_information!(NetworkEntry::ChannelClosed);
                            agent.state = AgentState::Terminate;
                            return;
                        }
                    }
                }
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
            }
            agent.control_channel_sender.send(ControlAckPacket::new()).await;
            return;
        }
    }

    async fn process_task(agent: &Arc<RwLock<Agent>>) -> Result<(), LogEntry> {
        let task_info = Self::receive_task(agent).await?;
        let result = Self::waiting_inference(agent, &task_info).await
            .map_err(|err| err.message);
        let task_result = TaskResult::new(result);
        Self::notice_complete(agent, &task_result).await?;
        Self::transfer_result(agent, &task_info).await?;
        Ok(())
    }

    async fn receive_task(agent: &Arc<RwLock<Agent>>) -> Result<TaskInfo, LogEntry> {
        let task_info = Self::receive_task_info(agent).await?;
        let previous_task_uuid = agent.read().await.previous_task_uuid;
        let need_receive_model = if let Some(previous_task_uuid) = previous_task_uuid {
            previous_task_uuid != task_info.uuid
        } else {
            true
        };
        if need_receive_model {
            let model_folder = PathBuf::from("./SavedModel");
            Self::receive_file(agent, &model_folder).await?;
            agent.write().await.previous_task_uuid = Some(task_info.uuid);
        }
        let media_folder = PathBuf::from("./SavedFile");
        Self::receive_file(agent, &media_folder).await?;
        Ok(task_info)
    }

    async fn receive_task_info(agent: &Arc<RwLock<Agent>>) -> Result<TaskInfo, LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.task_info_packet).await;
        }
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        let task_info = loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.task_info_packet.recv() => {
                        let packet = packet
                            .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        break serde_json::from_slice::<TaskInfo>(packet.as_data_byte())
                            .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        };
        if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
            data_channel_sender.send(TaskInfoAckPacket::new()).await;
        } else {
            Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
        }
        Ok(task_info)
    }

    async fn receive_file(agent: &Arc<RwLock<Agent>>, save_folder: &PathBuf) -> Result<(), LogEntry> {
        let file_header = Self::receive_file_header(agent).await?;
        let file_body = Self::receive_file_body(agent, &file_header).await?;
        Self::create_file(file_header, file_body, save_folder).await?;
        Ok(())
    }

    async fn receive_file_header(agent: &Arc<RwLock<Agent>>) -> Result<FileHeader, LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.file_header_packet).await;
        }
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
	    let file_header = loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.file_header_packet.recv() => {
                        let packet = packet
                            .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        break serde_json::from_slice::<FileHeader>(packet.as_data_byte())
                            .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(FileHeaderAckPacket::new()).await;
        } else {
            Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
        }
        Ok(file_header)
    }

    async fn receive_file_body(agent: &Arc<RwLock<Agent>>, file_header: &FileHeader) -> Result<Vec<Vec<u8>>, LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.file_body_packet).await;
        }
        let config = Config::now().await;
        let mut file_block: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut missing_blocks = Vec::new();
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.data_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_body_packet.recv() => {
                        let packet = packet
                            .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        let (sequence_bytes, file_body) = packet.data.split_at(size_of::<usize>());
                        let sequence_bytes = sequence_bytes.try_into()
                            .map_err(|_| error_entry!(MiscEntry::InvalidPacket))?;
                        let sequence_number = usize::from_be_bytes(sequence_bytes);
                        file_block.insert(sequence_number, Vec::from(file_body));
                        timer = Instant::now();
                        continue;
                    }
                    packet = data_channel_receiver.file_transfer_end_packet.recv() => {
                        clear_unbounded_channel(&mut data_channel_receiver.file_transfer_end_packet).await;
                        let _ = packet
                            .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        for sequence_number in 0..file_header.packet_count {
                            if !file_block.contains_key(&sequence_number) {
                                missing_blocks.push(sequence_number);
                            }
                        }
                        timer = Instant::now();
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
            if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                if missing_blocks.len() != 0_usize {
                    let missing_blocks = mem::take(&mut missing_blocks);
                    let result = FileTransferResult::new(Some(missing_blocks));
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                } else {
                    let result = FileTransferResult::new(None);
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
                    data_channel_sender.send(FileTransferResultPacket::new(result_data)).await;
                    let mut sorted_blocks: Vec<Vec<u8>> = Vec::with_capacity(file_header.packet_count);
                    for index in 0..file_header.packet_count {
                        if let Some(block) = file_block.remove(&index) {
                            sorted_blocks.push(block);
                        } else {
                            Err(error_entry!(MiscEntry::MissingFileBlockError))?
                        }
                    }
                    return Ok(sorted_blocks);
                };
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        }
    }

    async fn create_file(file_header: FileHeader, file_body: Vec<Vec<u8>>, saved_folder: &PathBuf) -> Result<(), LogEntry> {
        let saved_path = saved_folder.join(file_header.file_name);
        let mut file = File::create(&saved_path).await
            .map_err(|err| error_entry!(IOEntry::CreateFileError(saved_path.display(), err)))?;
        for chunk in file_body {
            file.write_all(&chunk).await
                .map_err(|err| error_entry!(IOEntry::WriteFileError(saved_path.display(), err)))?;
        }
        Ok(())
    }

    async fn waiting_inference(agent: &Arc<RwLock<Agent>>, task_info: &TaskInfo) -> Result<(), LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.still_process_packet).await;
        }
        let config = Config::now().await;
        let task_info = task_info.clone();
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let join_handle = tokio::spawn(Self::inference(task_info));
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if join_handle.is_finished() {
                break join_handle.await
                    .map_err(|err| error_entry!(SystemEntry::TaskPanickedError(err)))?;
            }
            let mut agent = agent.write().await;
            if let Some(data_channel_receiver) = agent.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.still_process_packet.recv() => {
                        let _ = packet.ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        clear_unbounded_channel(&mut data_channel_receiver.still_process_packet).await;
                        timer = Instant::now();
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
            if let Some(data_channel_sender) = agent.data_channel_sender.as_mut() {
                data_channel_sender.send(StillProcessAckPacket::new()).await;
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        }
    }

    async fn inference(task_info: TaskInfo) -> Result<(), LogEntry> {
        let inference_argument = task_info.inference_argument;
        let model_path = PathBuf::from(format!("./SavedModel/{}", task_info.model_file_name));
        let media_path = PathBuf::from(format!("./SavedFile/{}", task_info.media_file_name));
        match (inference_argument.model_type, media_path.extension().and_then(OsStr::to_str)) {
            (ModelType::Ultralytics, Some("png") | Some("jpg") | Some("jpeg")) =>
                InferenceManager::ultralytics_inference_image(inference_argument, model_path, media_path).await,
            (ModelType::Ultralytics, Some("mp4")) =>
                InferenceManager::ultralytics_inference_video(inference_argument, model_path, media_path).await,
            (ModelType::YOLOv4, Some("png") | Some("jpg") | Some("jpeg")) =>
                InferenceManager::yolov4_inference_picture(inference_argument, model_path, media_path).await,
            (ModelType::YOLOv4, Some("mp4")) =>
                InferenceManager::yolov4_inference_video(inference_argument, model_path, media_path).await,
            (ModelType::YOLOv7, Some("png") | Some("jpg") | Some("jpeg")) =>
                InferenceManager::yolov7_inference_picture(inference_argument, model_path, media_path).await,
            (ModelType::YOLOv7, Some("mp4")) =>
                InferenceManager::yolov7_inference_video(inference_argument, model_path, media_path).await,
            _ => Err(error_entry!(TaskEntry::UnSupportFileType(task_info.uuid))),
        }
    }

    async fn notice_complete(agent: &Arc<RwLock<Agent>>, task_result: &TaskResult) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let task_result_data = serde_json::to_vec(task_result)
            .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
        let mut polling_times = 0_u32;
        let timer = Instant::now();
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                let data_channel_sender = agent.data_channel_sender.as_mut()
                    .ok_or(warning_entry!(NetworkEntry::DataChannelNotReady))?;
                data_channel_sender.send(TaskResultPacket::new(task_result_data.clone())).await;
                polling_times += 1;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.task_result_ack_packet.recv() => {
                        clear_unbounded_channel(&mut data_channel_receiver.task_result_ack_packet).await;
                        if packet.is_some() {
                            return Ok(())
                        } else {
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        }
    }

    async fn transfer_result(agent: &Arc<RwLock<Agent>>, task_info: &TaskInfo) -> Result<(), LogEntry> {
        let file_name = task_info.media_file_name.clone();
        let file_path = PathBuf::from(format!("./Result/{}", file_name));
        let file_body = Self::read_file(agent, &file_path).await?;
        Self::transfer_file_header(agent, &file_name, file_body.len()).await?;
        Self::transfer_file_body(agent, file_body).await?;
        Ok(())
    }

    async fn read_file(agent: &Arc<RwLock<Agent>>, file_path: &PathBuf) -> Result<Vec<Vec<u8>>, LogEntry> {
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut packets = Vec::new();
        let mut file = File::open(file_path.clone()).await
            .map_err(|err| error_entry!(IOEntry::ReadFileError(file_path.display(), err)))?;
        while agent.read().await.state != AgentState::Terminate {
            let bytes_read = file.read(&mut buffer).await
                .map_err(|err| error_entry!(IOEntry::ReadFileError(file_path.display(), err)))?;
            if bytes_read == 0 {
                return Ok(packets);
            }
            let mut data = sequence_number.to_be_bytes().to_vec();
            data.extend_from_slice(&buffer[..bytes_read]);
            packets.push(data);
            sequence_number += 1;
        }
        Err(information_entry!(SystemEntry::Cancel))?
    }

    async fn transfer_file_header(agent: &Arc<RwLock<Agent>>, file_name: &String, packet_count: usize) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let file_header = FileHeader::new(file_name.clone(), packet_count);
        let file_header_data = serde_json::to_vec(&file_header)
            .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while agent.read().await.state != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                let data_channel_sender = agent.data_channel_sender.as_mut()
                    .ok_or(warning_entry!(NetworkEntry::DataChannelNotReady))?;
                data_channel_sender.send(FileHeaderPacket::new(file_header_data.clone())).await;
                polling_times += 1;
            }
            let mut agent = agent.write().await;
            if let Some(data_channel_receiver) = agent.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.file_header_ack_packet.recv() => {
                        clear_unbounded_channel(&mut data_channel_receiver.file_header_ack_packet).await;
                        if packet.is_some() {
                            return Ok(())
                        } else {
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        }
        Err(information_entry!(SystemEntry::Cancel))
    }

    async fn transfer_file_body(agent: &Arc<RwLock<Agent>>, file_body: Vec<Vec<u8>>) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let mut timer = Instant::now();
        let mut require_send: Vec<usize> = (0..file_body.len()).collect();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        while agent.read().await.state != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            for chunk in &require_send {
                let data = file_body.get(*chunk)
                    .ok_or(error_entry!(MiscEntry::MissingFileBlockError))?;
                let mut agent = agent.write().await;
                let data_channel_sender = agent.data_channel_sender.as_mut()
                    .ok_or(warning_entry!(NetworkEntry::DataChannelNotReady))?;
                data_channel_sender.send(FileBodyPacket::new(data.clone())).await;
            }
            if !require_send.is_empty() {
                let mut agent = agent.write().await;
                let data_channel_sender = agent.data_channel_sender.as_mut()
                    .ok_or(warning_entry!(NetworkEntry::DataChannelNotReady))?;
                data_channel_sender.send(FileTransferEndPacket::new()).await;
                require_send = Vec::new();
                timer = Instant::now();
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_transfer_result_packet.recv() => {
                        clear_unbounded_channel(&mut data_channel_receiver.file_transfer_result_packet).await;
                        let packet = packet.ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                        let file_transfer_result = serde_json::from_slice::<FileTransferResult>(packet.as_data_byte())
                            .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                        timer = Instant::now();
                        match file_transfer_result.into() {
                            Some(missing_chunks) => require_send = missing_chunks,
                            None => return Ok(()),
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        }
        Err(information_entry!(SystemEntry::Cancel))?
    }

    async fn idle(agent: &Arc<RwLock<Agent>>, idle_duration: Duration) -> Result<(), LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.alive_packet).await;
        }
        let config = Config::now().await;
        let timer = Instant::now();
        while timer.elapsed() <= idle_duration {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            let mut agent = agent.write().await;
            if let Some(data_channel_receiver) = agent.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    _ = data_channel_receiver.alive_packet.recv() =>
                        clear_unbounded_channel(&mut data_channel_receiver.alive_packet).await,
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
            if let Some(data_channel_sender) = agent.data_channel_sender.as_mut() {
                data_channel_sender.send(AliveAckPacket::new()).await;
            } else {
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        }
        Ok(())
    }

    #[allow(unused_assignments)]
    async fn create_data_channel(agent: &Arc<RwLock<Agent>>) -> Result<(), LogEntry> {
        {
            let mut agent = agent.write().await;
            clear_unbounded_channel(&mut agent.control_channel_receiver.data_channel_port_packet).await;
        }
        let config = Config::now().await;
        let mut port: Option<u16> = None;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::Terminate;
                Err(information_entry!(NetworkEntry::ControlChannelTimeout))?;
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
                                logging_error!(MiscEntry::InvalidPacket);
                                continue;
                            }
                        } else {
                            agent.state = AgentState::Terminate;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
            if let Some(port) = port {
                let address = format!("{}:{}", config.management_address, port);
                let tcp_stream = TcpStream::connect(&address).await
                    .map_err(|err| error_entry!(NetworkEntry::EstablishConnectionError(err)))?;
                let socket_stream = SocketStream::new(tcp_stream);
                let (data_channel_sender, data_channel_receiver) = DataChannel::new(socket_stream);
                let mut agent = agent.write().await;
                agent.data_channel_sender = Some(data_channel_sender);
                agent.data_channel_receiver = Some(data_channel_receiver);
                break;
            }
        }
        logging_information!(NetworkEntry::CreateDataChannelSuccess);
        Ok(())
    }
}
