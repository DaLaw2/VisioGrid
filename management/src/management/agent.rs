use crate::connection::channel::control_channel_receiver::ControlChannelReceiver;
use crate::connection::channel::control_channel_sender::ControlChannelSender;
use crate::connection::channel::data_channel_receiver::DataChannelReceiver;
use crate::connection::channel::data_channel_sender::DataChannelSender;
use crate::connection::channel::ControlChannel;
use crate::connection::channel::DataChannel;
use crate::connection::packet::agent_info_ack_packet::AgentInfoAckPacket;
use crate::connection::packet::alive_packet::AlivePacket;
use crate::connection::packet::control_packet::ControlPacket;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;
use crate::connection::packet::file_body_packet::FileBodyPacket;
use crate::connection::packet::file_header_ack_packet::FileHeaderAckPacket;
use crate::connection::packet::file_header_packet::FileHeaderPacket;
use crate::connection::packet::file_transfer_end_packet::FileTransferEndPacket;
use crate::connection::packet::file_transfer_result_packet::FileTransferResultPacket;
use crate::connection::packet::performance_ack_packet::PerformanceAckPacket;
use crate::connection::packet::still_process_packet::StillProcessPacket;
use crate::connection::packet::task_result_ack_packet::TaskResultAckPacket;
use crate::connection::packet::task_info_packet::TaskInfoPacket;
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::SocketStream;
use crate::management::agent_manager::AgentManager;
use crate::management::task_manager::TaskManager;
use crate::management::utils::agent_information::AgentInformation;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::file_header::FileHeader;
use crate::management::utils::file_transfer_result::FileTransferResult;
use crate::management::utils::inference_task::InferenceTask;
use crate::management::utils::performance::Performance;
use crate::management::utils::task_result::TaskResult;
use crate::utils::clear_unbounded_channel;
use crate::utils::config::Config;
use crate::utils::logging::*;
use crate::utils::port_pool::PortPool;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

pub struct Agent {
    uuid: Uuid,
    state: AgentState,
    information: AgentInformation,
    idle_unused: Performance,
    realtime_usage: Performance,
    previous_task_uuid: Option<Uuid>,
    inference_task: VecDeque<InferenceTask>,
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
                packet = control_channel_receiver.agent_info_packet.recv(), if agent_information.is_none() => {
                    let packet = packet
                        .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                    let information = serde_json::from_slice::<AgentInformation>(packet.as_data_byte())
                        .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                    agent_information = Some(information);
                    control_channel_sender.send(AgentInfoAckPacket::new()).await;
                },
                packet = control_channel_receiver.performance_packet.recv() => {
                    let packet = packet
                        .ok_or(information_entry!(NetworkEntry::ChannelClosed))?;
                    let realtime_usage = serde_json::from_slice::<Performance>(packet.as_data_byte())
                        .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                    let information = agent_information
                        .ok_or(error_entry!(MiscEntry::WrongDeliverOrder))?;
                    control_channel_sender.send(PerformanceAckPacket::new()).await;
                    let residual_usage = Performance::calc_residual_usage(&information, &realtime_usage);
                    let agent = Self {
                        uuid,
                        state: AgentState::CreateDataChannel,
                        information,
                        idle_unused: residual_usage,
                        realtime_usage,
                        previous_task_uuid: None,
                        inference_task: VecDeque::new(),
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
        Err(information_entry!(NetworkEntry::ControlChannelTimeout))
    }

    pub async fn add_task(agent: Arc<RwLock<Agent>>, inference_task: InferenceTask) {
        agent.write().await.inference_task.push_back(inference_task);
    }

    pub async fn run(agent: Arc<RwLock<Agent>>) {
        let for_performance = agent.clone();
        let for_management = agent;
        tokio::spawn(async move {
            Agent::performance(for_performance).await
        });
        tokio::spawn(async move {
            Agent::management(for_management).await
        });
    }

    async fn performance(agent: Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        loop {
            if agent.read().await.state == AgentState::Terminate {
                return;
            }
            if timer.elapsed() > timeout_duration {
                logging_information!(uuid, NetworkEntry::ControlChannelTimeout, "");
                break;
            }
            let mut agent = agent.write().await;
            select! {
                biased;
                packet = agent.control_channel_receiver.performance_packet.recv() => {
                    if let Some(packet) = packet {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.performance_packet).await;
                        match serde_json::from_slice::<Performance>(packet.as_data_byte()) {
                            Ok(performance) => {
                                agent.realtime_usage = performance;
                                agent.control_channel_sender.send(PerformanceAckPacket::new()).await;
                                timer = Instant::now();
                            }
                            Err(err) => {
                                logging_error!(uuid, IOEntry::SerdeDeserializeError(err), "");
                                continue;
                            }
                        }
                    } else {
                        logging_information!(uuid, NetworkEntry::ChannelClosed, "");
                        break;
                    }
                },
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
            }
        }
        agent.write().await.state = AgentState::Terminate;
    }

    async fn management(agent: Arc<RwLock<Agent>>) {
        let config = Config::now().await;
        loop {
            let state = agent.read().await.state;
            match state {
                AgentState::CreateDataChannel => {
                    Self::send_state(&agent, AgentState::CreateDataChannel).await;
                    Self::create_data_channel(&agent).await;
                }
                AgentState::Terminate => {
                    Self::send_state(&agent, AgentState::Terminate).await;
                    Self::terminate(&agent).await;
                    return;
                }
                _ => {
                    let inference_task = agent.write().await.inference_task.pop_front();
                    if let Some(mut inference_task) = inference_task {
                        let state = AgentState::ProcessTask;
                        agent.write().await.state = state;
                        Self::send_state(&agent, state).await;
                        let result = Self::process_task(&agent, &mut inference_task).await;
                        inference_task.error = result;
                        TaskManager::submit_inference_task(inference_task).await;
                    } else {
                        let state = AgentState::Idle(config.agent_idle_duration);
                        agent.write().await.state = state;
                        Self::send_state(&agent, state).await;
                        Self::idle(&agent).await;
                    }
                }
            }
        }
    }

    async fn send_state(agent: &Arc<RwLock<Agent>>, state: AgentState) {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        match serde_json::to_vec(&state) {
            Ok(control_state_data) => loop {
                if timer.elapsed() > timeout_duration {
                    logging_information!(uuid, NetworkEntry::ControlChannelTimeout, "");
                    break;
                }
                if timer.elapsed() > polling_times * polling_interval {
                    let mut agent = agent.write().await;
                    agent.control_channel_sender.send(ControlPacket::new(control_state_data.clone())).await;
                    polling_times += 1;
                }
                let mut agent = agent.write().await;
                select! {
                    biased;
                    packet = agent.control_channel_receiver.control_ack_packet.recv() => {
                        clear_unbounded_channel(&mut agent.control_channel_receiver.control_ack_packet).await;
                        if packet.is_some() {
                            return;
                        } else {
                            logging_information!(uuid, NetworkEntry::ChannelClosed, "");
                            break;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            }
            Err(err) => logging_error!(uuid, IOEntry::SerdeSerializeError(err), "")
        }
        agent.write().await.state = AgentState::Terminate;
    }

    async fn process_task(agent: &Arc<RwLock<Agent>>, inference_task: &mut InferenceTask) -> Result<(), String> {
        let uuid = agent.read().await.uuid;
        if let Err(entry) = Self::transfer_task(&agent, inference_task).await {
            logging_entry!(uuid, entry.clone());
            return Err(entry.message);
        }
        if let Err(entry) = Self::waiting_complete(&agent).await {
            logging_entry!(uuid, entry.clone());
            return Err(entry.message);
        }
        if let Err(entry) = Self::receive_result(&agent, inference_task).await {
            logging_entry!(uuid, entry.clone());
            return Err(entry.message);
        }
        Ok(())
    }

    async fn transfer_task(agent: &Arc<RwLock<Agent>>, inference_task: &InferenceTask) -> Result<(), LogEntry> {
        if agent.write().await.data_channel_sender.is_some() {
            let need_transfer_model = if let Some(last_task_uuid) = &agent.read().await.previous_task_uuid {
                inference_task.task_uuid != *last_task_uuid
            } else {
                true
            };
            Agent::transfer_task_info(agent, &inference_task).await?;
            if need_transfer_model {
                Agent::transfer_file(agent, &inference_task.model_file_name, &inference_task.model_file_path).await?;
                agent.write().await.previous_task_uuid = Some(inference_task.task_uuid);
            }
            Agent::transfer_file(agent, &inference_task.media_file_name, &inference_task.media_file_path).await?;
        } else {
            agent.write().await.state = AgentState::CreateDataChannel;
        }
        Ok(())
    }

    async fn transfer_task_info(agent: &Arc<RwLock<Agent>>, inference_task: &InferenceTask) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let task_info = inference_task.as_task_info();
        let task_info_data = serde_json::to_vec(&task_info)
            .map_err(|err| error_entry!(IOEntry::SerdeSerializeError(err)))?;
        let timer = Instant::now();
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        while agent.read().await.state != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                if let Some(data_channel_sender) = agent.data_channel_sender.as_mut() {
                    data_channel_sender.send(TaskInfoPacket::new(task_info_data.clone())).await;
                    polling_times += 1;
                } else {
                    agent.state = AgentState::CreateDataChannel;
                    Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
                }
            }
            let mut agent = agent.write().await;
            match agent.data_channel_receiver.as_mut() {
                Some(data_channel_receiver) => {
                    select! {
                        packet = data_channel_receiver.task_info_ack_packet.recv() => {
                            clear_unbounded_channel(&mut data_channel_receiver.task_info_ack_packet).await;
                            if packet.is_some() {
                                return Ok(())
                            } else {
                                agent.state = AgentState::CreateDataChannel;
                                Err(information_entry!(NetworkEntry::ChannelClosed))?;
                            }
                        }
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                    }
                }
                None => {
                    agent.state = AgentState::CreateDataChannel;
                    Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
                }
            }
        }
        Err(information_entry!(SystemEntry::Cancel))
    }

    async fn transfer_file(agent: &Arc<RwLock<Agent>>, file_name: &String, file_path: &PathBuf) -> Result<(), LogEntry> {
        let file_body = Self::read_file(agent, file_path).await?;
        Self::transfer_file_header(agent, file_name, file_body.len()).await?;
        Self::transfer_file_body(agent, file_body).await
    }

    async fn read_file(agent: &Arc<RwLock<Agent>>, file_path: &PathBuf) -> Result<Vec<Vec<u8>>, LogEntry> {
        let mut sequence_number = 0_usize;
        let mut buffer = vec![0; 1_048_576];
        let mut packets = Vec::new();
        let mut file = File::open(file_path.clone()).await
            .map_err(|err| error_entry!(IOEntry::ReadFileError(file_path.display(), err)))?;
        loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?
            }
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
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                let mut agent = agent.write().await;
                if let Some(data_channel_sender) = agent.data_channel_sender.as_mut() {
                    data_channel_sender.send(FileHeaderPacket::new(file_header_data.clone())).await
                } else {
                    agent.state = AgentState::CreateDataChannel;
                    Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
                }
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
                            agent.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                agent.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        }
        Err(information_entry!(SystemEntry::Cancel))
    }

    async fn transfer_file_body(agent: &Arc<RwLock<Agent>>, file_body: Vec<Vec<u8>>) -> Result<(), LogEntry> {
        let config = Config::now().await;
        let mut require_send: Vec<usize> = (0..file_body.len()).collect();
        let mut timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.file_transfer_timeout);
        while agent.read().await.state != AgentState::Terminate {
            if timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            for chunk in &require_send {
                if let Some(data) = file_body.get(*chunk) {
                    if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                        data_channel_sender.send(FileBodyPacket::new(data.clone())).await;
                    } else {
                        agent.write().await.state = AgentState::CreateDataChannel;
                        Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
                    }
                } else {
                    Err(error_entry!(MiscEntry::MissingFileBlockError))?
                }
            }
            if !require_send.is_empty() {
                if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                    data_channel_sender.send(FileTransferEndPacket::new()).await;
                } else {
                    agent.write().await.state = AgentState::CreateDataChannel;
                    Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
                }
                timer = Instant::now();
                require_send = Vec::new();
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_transfer_result_packet.recv() => {
                        match packet {
                            Some(packet) => {
                                clear_unbounded_channel(&mut data_channel_receiver.file_transfer_result_packet).await;
                                let file_transfer_result = serde_json::from_slice::<FileTransferResult>(packet.as_data_byte())
                                    .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                                timer = Instant::now();
                                match file_transfer_result.into() {
                                    Some(missing_chunks) => require_send = missing_chunks,
                                    None => return Ok(()),
                                }
                            }
                            None => {
                                agent.write().await.state = AgentState::CreateDataChannel;
                                Err(information_entry!(NetworkEntry::ChannelClosed))?;
                            }
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
                }
            } else {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?
            }
        }
        Err(information_entry!(SystemEntry::Cancel))?
    }

    async fn waiting_complete(agent: &Arc<RwLock<Agent>>) -> Result<(), LogEntry> {
        if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
            clear_unbounded_channel(&mut data_channel_receiver.task_result_packet).await;
        }
        let config = Config::now().await;
        let mut polling_times = 0_u32;
        let polling_timer = Instant::now();
        let polling_interval = Duration::from_millis(config.polling_interval);
        let mut timeout_timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let task_result = loop {
            if agent.read().await.state == AgentState::Terminate {
                Err(information_entry!(SystemEntry::Cancel))?;
            }
            if timeout_timer.elapsed() > timeout_duration {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if polling_timer.elapsed() > polling_times * polling_interval {
                if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                    data_channel_sender.send(StillProcessPacket::new()).await;
                } else {
                    agent.write().await.state = AgentState::CreateDataChannel;
                    Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
                }
                polling_times += 1;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.still_process_ack_packet.recv() => {
                        if packet.is_some() {
                            clear_unbounded_channel(&mut data_channel_receiver.still_process_ack_packet).await;
                            timeout_timer = Instant::now();
                        } else {
                            agent.write().await.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    packet = data_channel_receiver.task_result_packet.recv() => {
                        if let Some(packet) = &packet {
                            match serde_json::from_slice::<TaskResult>(packet.as_data_byte()) {
                                Ok(task_result) => break task_result.into(),
                                Err(err) => Err(error_entry!(IOEntry::SerdeDeserializeError(err)))?,
                            }
                        } else {
                            agent.write().await.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        };
        if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
            data_channel_sender.send(TaskResultAckPacket::new()).await;
            task_result.map_err(|err| error_entry!(TaskEntry::AgentProcessingError(err)))?;
        } else {
            agent.write().await.state = AgentState::CreateDataChannel;
            Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
        }
        Ok(())
    }

    async fn receive_result(agent: &Arc<RwLock<Agent>>, inference_task: &InferenceTask) -> Result<(), LogEntry> {
        let uuid = inference_task.task_uuid.to_string();
        #[cfg(target_os = "linux")]
        let save_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let save_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let file_header = Self::receive_file_header(agent.clone()).await?;
        let file_body = Self::receive_file_body(agent.clone(), &file_header).await?;
        Self::create_file(file_header, file_body, save_folder).await?;
        Ok(())
    }

    async fn receive_file_header(agent: Arc<RwLock<Agent>>) -> Result<FileHeader, LogEntry> {
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
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    packet = data_channel_receiver.file_header_packet.recv() => {
                        if let Some(packet) = packet {
                            clear_unbounded_channel(&mut data_channel_receiver.file_header_packet).await;
                            break serde_json::from_slice::<FileHeader>(packet.as_data_byte())
                                .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
                        } else {
                            agent.write().await.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        };
        if let Some(data_channel_sender) = &mut agent.write().await.data_channel_sender {
            data_channel_sender.send(FileHeaderAckPacket::new()).await;
        } else {
            agent.write().await.state = AgentState::CreateDataChannel;
            Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
        }
        Ok(file_header)
    }

    async fn receive_file_body(agent: Arc<RwLock<Agent>>, file_header: &FileHeader) -> Result<Vec<Vec<u8>>, LogEntry> {
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
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if let Some(data_channel_receiver) = agent.write().await.data_channel_receiver.as_mut() {
                select! {
                    biased;
                    packet = data_channel_receiver.file_body_packet.recv() => {
                        if let Some(packet) = packet {
                            clear_unbounded_channel(&mut data_channel_receiver.file_body_packet).await;
                            timer = Instant::now();
                            let (sequence_bytes, file_body) = packet.data.split_at(size_of::<usize>());
                            let sequence_bytes = sequence_bytes.try_into()
                                .map_err(|_| error_entry!(MiscEntry::InvalidPacket))?;
                            let sequence_number = usize::from_be_bytes(sequence_bytes);
                            file_block.insert(sequence_number, Vec::from(file_body));
                            continue;
                        } else {
                            agent.write().await.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    packet = data_channel_receiver.file_transfer_end_packet.recv() => {
                        clear_unbounded_channel(&mut data_channel_receiver.file_transfer_end_packet).await;
                        if packet.is_some() {
                            for sequence_number in 0..file_header.packet_count {
                                if !file_block.contains_key(&sequence_number) {
                                    missing_blocks.push(sequence_number);
                                }
                            }
                            timer = Instant::now();
                        } else {
                            agent.write().await.state = AgentState::CreateDataChannel;
                            Err(information_entry!(NetworkEntry::ChannelClosed))?;
                        }
                    }
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            } else {
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
            if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                if missing_blocks.len() != 0_usize {
                    let missing_blocks = mem::take(&mut missing_blocks);
                    let result = FileTransferResult::new(Some(missing_blocks));
                    let result_data = serde_json::to_vec(&result)
                        .map_err(|err| error_entry!(IOEntry::SerdeDeserializeError(err)))?;
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
                agent.write().await.state = AgentState::CreateDataChannel;
                Err(warning_entry!(NetworkEntry::DataChannelNotReady))?;
            }
        }
    }

    async fn create_file(file_header: FileHeader, file_body: Vec<Vec<u8>>, saved_folder: PathBuf) -> Result<(), LogEntry> {
        let saved_path = saved_folder.join(file_header.file_name);
        let mut file = File::create(&saved_path).await
            .map_err(|err| error_entry!(IOEntry::CreateFileError(saved_path.display(), err)))?;
        for chunk in file_body {
            file.write_all(&chunk).await
                .map_err(|err| error_entry!(IOEntry::CreateFileError(saved_path.display(), err)))?;
        }
        Ok(())
    }

    async fn idle(agent: &Arc<RwLock<Agent>>) {
        if let Some(inference_task) = TaskManager::steal_task(agent.clone()).await {
            Agent::add_task(agent.clone(), inference_task).await;
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
            while agent.read().await.state != AgentState::Terminate && timer.elapsed() <= idle_duration {
                if timeout_timer.elapsed() > timeout_duration {
                    agent.write().await.state = AgentState::CreateDataChannel;
                    logging_information!(uuid, NetworkEntry::DataChannelTimeout, "");
                    return;
                }
                if timer.elapsed() > polling_times * polling_interval {
                    if let Some(data_channel_sender) = agent.write().await.data_channel_sender.as_mut() {
                        data_channel_sender.send(AlivePacket::new()).await
                    } else {
                        agent.write().await.state = AgentState::CreateDataChannel;
                        logging_warning!(uuid, NetworkEntry::DataChannelNotReady, "");
                        return;
                    }
                    polling_times += 1;
                }
                let mut agent = agent.write().await;
                if let Some(data_channel_receiver) = agent.data_channel_receiver.as_mut() {
                    select! {
                        biased;
                        packet = data_channel_receiver.alive_ack_packet.recv() => {
                            if packet.is_some() {
                                clear_unbounded_channel(&mut data_channel_receiver.alive_ack_packet).await;
                                timeout_timer = Instant::now();
                            } else {
                                agent.state = AgentState::CreateDataChannel;
                                logging_information!(uuid, NetworkEntry::ChannelClosed, "");
                                return;
                            }
                        },
                        _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                    }
                } else {
                    agent.state = AgentState::CreateDataChannel;
                    logging_warning!(uuid, NetworkEntry::DataChannelNotReady, "");
                    return;
                }
            }
        }
    }

    async fn create_data_channel(agent: &Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        match Self::create_listener(agent).await {
            Ok((listener, port)) => {
                if let Err(entry) = Self::accept_connection(agent, listener, port).await {
                    logging_entry!(uuid, entry);
                }
                agent.write().await.state = AgentState::None;
            }
            Err(entry) => logging_entry!(uuid, entry)
        }
    }

    async fn create_listener(agent: &Arc<RwLock<Agent>>) -> Result<(TcpListener, u16), LogEntry> {
        loop {
            if agent.read().await.state == AgentState::Terminate {
                return Err(information_entry!(SystemEntry::Cancel));
            }
            let port = PortPool::allocate_port().await
                .ok_or(warning_entry!(SystemEntry::NoAvailablePort))?;
            match TcpListener::bind(format!("0.0.0.0:{port}")).await {
                Ok(listener) => break Ok((listener, port)),
                Err(err) => {
                    PortPool::free_port(port).await;
                    Err(error_entry!(NetworkEntry::BindPortError(err)))?;
                }
            }
        }
    }

    async fn accept_connection(agent: &Arc<RwLock<Agent>>, listener: TcpListener, port: u16) -> Result<(), LogEntry> {
        let uuid = agent.read().await.uuid;
        let config = Config::now().await;
        let timer = Instant::now();
        let timeout_duration = Duration::from_secs(config.control_channel_timeout);
        let mut polling_times = 0_u32;
        let polling_interval = Duration::from_millis(config.polling_interval);
        let (tcp_stream, _) = loop {
            if agent.read().await.state == AgentState::Terminate {
                PortPool::free_port(port).await;
                Err(information_entry!(SystemEntry::Cancel))?
            }
            if timer.elapsed() > timeout_duration {
                PortPool::free_port(port).await;
                agent.write().await.state = AgentState::Terminate;
                Err(information_entry!(NetworkEntry::DataChannelTimeout))?;
            }
            if timer.elapsed() > polling_times * polling_interval {
                let port_data = port.to_be_bytes().to_vec();
                agent.write().await.control_channel_sender.send(DataChannelPortPacket::new(port_data)).await;
                polling_times += 1;
            }
            select! {
                biased;
                connection = listener.accept() => {
                    match connection {
                        Ok(connection) => break connection,
                        Err(err) => Err(error_entry!(NetworkEntry::EstablishConnectionError(err)))?,
                    }
                }
                _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue
            }
        };
        let socket_stream = SocketStream::new(tcp_stream);
        let (data_channel_sender, data_channel_receiver) = DataChannel::new(uuid, socket_stream);
        let mut agent = agent.write().await;
        agent.data_channel_sender = Some(data_channel_sender);
        agent.data_channel_receiver = Some(data_channel_receiver);
        logging_information!(uuid, NetworkEntry::CreateDataChannelSuccess, "");
        Ok(())
    }

    pub async fn terminate(agent: &Arc<RwLock<Agent>>) {
        let uuid = agent.read().await.uuid;
        logging_information!(uuid, SystemEntry::Terminating, "");
        let inference_task = {
            let mut agent = agent.write().await;
            agent.control_channel_sender.disconnect().await;
            agent.control_channel_receiver.disconnect().await;
            if let Some(data_channel_sender) = &mut agent.data_channel_sender {
                data_channel_sender.disconnect().await;
            }
            if let Some(data_channel_receiver) = &mut agent.data_channel_receiver {
                data_channel_receiver.disconnect().await;
            }
            mem::take(&mut agent.inference_task)
        };
        TaskManager::redistribute_task(inference_task).await;
        AgentManager::remove_agent(uuid).await;
        logging_information!(uuid, SystemEntry::TerminateComplete, "");
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

    pub fn inference_tasks(&mut self) -> &mut VecDeque<InferenceTask> {
        &mut self.inference_task
    }
}
