use crate::management::utils::agent_information::AgentInformation;
use crate::management::utils::performance::Performance;
use crate::utils::log_entry::system::SystemEntry;
use crate::utils::logging::*;
use lazy_static::lazy_static;
use std::process::Command;
use sysinfo::System;
use tokio::process::Command as AsyncCommand;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::sleep;

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
        logging_console!(information_entry!(SystemEntry::Online));
    }

    pub async fn terminate() {
        Self::instance_mut().await.terminate = true;
    }

    fn system_info() -> AgentInformation {
        let sys = System::new_all();
        let host_name = System::host_name().expect("Fail to get system information.");
        let os_name = if cfg!(target_os = "windows") {
            let long_os_version = System::long_os_version().expect("Fail to get system information.");
            let kernel_version = System::kernel_version().expect("Fail to get system information.");
            format!("{} build {}", long_os_version, kernel_version)
        } else if cfg!(target_os = "linux") {
            let long_os_version = System::long_os_version().expect("Fail to get system information.");
            let kernel_version = System::kernel_version().expect("Fail to get system information.");
            format!("{} {}", long_os_version, kernel_version)
        } else {
            System::long_os_version().expect("Fail to get system information.")
        };
        let cpu = sys.cpus().get(0).map(|cpu| cpu.brand())
            .expect("Fail to get system information.")
            .to_string();
        let cores = sys.physical_core_count().expect("Fail to get system information.");
        let ram = sys.total_memory() as f64;
        let gpu = Self::get_gpu_name().expect("Fail to get system information.");
        let vram = Self::get_vram_total().expect("Fail to get system information.") as f64;
        AgentInformation {
            host_name,
            os_name,
            cpu,
            cores,
            ram,
            gpu,
            vram,
        }
    }

    fn get_gpu_name() -> Result<String, String> {
        let gpu_name = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
            .map_err(|_| "Fail to get gpu information.".to_string())?;
        let gpu_name = String::from_utf8_lossy(&gpu_name.stdout).trim().to_string();
        Ok(gpu_name)
    }

    fn get_vram_total() -> Result<u64, String> {
        let vram_total = Command::new("nvidia-smi")
            .arg("--query-gpu=memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
            .map_err(|_| "Fail to get gpu information.".to_string())?;
        let vram_total = String::from_utf8_lossy(&vram_total.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Fail to parse gpu information.".to_string())?;
        Ok(vram_total * 1_048_576_u64)
    }

    async fn get_gpu_usage() -> Result<u64, String> {
        let gpu_usage = AsyncCommand::new("nvidia-smi")
            .arg("--query-gpu=utilization.gpu")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
            .map_err(|_| "Fail to get gpu information.".to_string())?;
        let gpu_usage = String::from_utf8_lossy(&gpu_usage.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Fail to parse gpu information.".to_string())?;
        Ok(gpu_usage)
    }

    async fn get_vram_used() -> Result<u64, String> {
        let vram_used = AsyncCommand::new("nvidia-smi")
            .arg("--query-gpu=memory.used")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
            .map_err(|_| "Fail to get gpu information.".to_string())?;
        let vram_used = String::from_utf8_lossy(&vram_used.stdout).trim().to_string()
            .parse::<u64>()
            .map_err(|_| "Fail to parse gpu information.".to_string())?;
        Ok(vram_used * 1_048_576_u64)
    }

    async fn update_performance() {
        let mut system = System::new_all();
        while !Self::instance().await.terminate {
            system.refresh_all();
            let cpu_usage = system.cpus().iter()
                .map(|core| core.cpu_usage() as f64)
                .sum::<f64>() / system.cpus().len() as f64;
            let ram_used = system.used_memory() as f64;
            let gpu_usage = Self::get_gpu_usage().await.unwrap_or_default() as f64;
            let vram_used = Self::get_vram_used().await.unwrap_or_default() as f64;
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
