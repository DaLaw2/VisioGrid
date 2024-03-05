use sysinfo::System;
use tokio::time::sleep;
use std::time::Duration;
use std::process::Command;
use lazy_static::lazy_static;
use tokio::process::Command as AsyncCommand;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::utils::config::Config;
use crate::management::utils::performance::Performance;
use crate::management::utils::agent_information::AgentInformation;

lazy_static! {
    static ref MONITOR: RwLock<Monitor> = RwLock::new(Monitor::new());
}

pub struct Monitor {
    information: AgentInformation,
    performance: Performance,
    terminate: bool,
}

impl Monitor {
    fn new() -> Self {
        Self {
            information: Self::system_info(),
            performance: Performance::default(),
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MONITOR.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MONITOR.write().await
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::update_performance().await;
        });
    }

    pub async fn terminate() {
        Self::instance_mut().await.terminate = true;
    }

    fn system_info() -> AgentInformation {
        let sys = System::new_all();
        let cpu = sys.cpus().get(0).map(|cpu| cpu.brand())
            .expect("Monitor: Fail to get system information.")
            .to_string();
        let gpu = Self::get_gpu_name().expect("Monitor: Fail to get system information.");
        let vram = Self::get_vram_total().expect("Monitor: Fail to get system information.");
        AgentInformation {
            host_name: System::host_name().expect("Monitor: Fail to get system information."),
            system_name: System::name().expect("Monitor: Fail to get system information."),
            cpu,
            cores: sys.physical_core_count().unwrap_or(0),
            ram: sys.total_memory(),
            gpu,
            vram,
        }
    }

    fn get_gpu_name() -> Result<String, String> {
        let gpu_name = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
            .map_err(|_| "Monitor: Fail to get gpu information.".to_string())?;
        let gpu_name = String::from_utf8_lossy(&gpu_name.stdout).trim().to_string();
        Ok(gpu_name)
    }

    fn get_vram_total() -> Result<u64, String> {
        let vram_total = Command::new("nvidia-smi")
            .arg("--query-gpu=memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
            .map_err(|_| "Monitor: Fail to get gpu information.".to_string())?;
        let vram_total = String::from_utf8_lossy(&vram_total.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Monitor: Fail to parse gpu information.".to_string())?;
        Ok(vram_total)
    }

    async fn get_gpu_usage() -> Result<u64, String> {
        let gpu_usage = AsyncCommand::new("nvidia-smi")
            .arg("--query-gpu=utilization.gpu")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
            .map_err(|_| "Monitor: Fail to get gpu information.".to_string())?;
        let gpu_usage = String::from_utf8_lossy(&gpu_usage.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Monitor: Fail to parse gpu information.".to_string())?;
        Ok(gpu_usage)
    }

    async fn get_vram_used() -> Result<u64, String> {
        let vram_used = AsyncCommand::new("nvidia-smi")
            .arg("--query-gpu=memory.used")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
            .map_err(|_| "Monitor: Fail to get gpu information.".to_string())?;
        let vram_used = String::from_utf8_lossy(&vram_used.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Monitor: Fail to parse gpu information.".to_string())?;
        Ok(vram_used)
    }

    async fn update_performance() {
        let mut system = System::new_all();
        while !Self::instance().await.terminate {
            system.refresh_all();
            let cpu_usage = system.cpus().iter()
                .map(|core| core.cpu_usage() as f64)
                .sum::<f64>() / system.cpus().len() as f64;
            let ram_used = system.used_memory();
            let gpu_usage = Self::get_gpu_usage().await.unwrap_or_default() as f64;
            let vram_used = Self::get_vram_used().await.unwrap_or_default();
            Self::instance_mut().await.performance = Performance::new(cpu_usage, ram_used, gpu_usage, vram_used);
            sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        }
    }

    pub async fn get_system_info() -> AgentInformation {
        Self::instance().await.information.clone()
    }

    pub async fn get_performance() -> Performance {
        Self::instance().await.performance.clone()
    }
}
