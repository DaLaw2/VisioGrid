use std::process::Command;
use lazy_static::lazy_static;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::management::utils::performance::Performance;
use crate::management::utils::agent_information::AgentInformation;

lazy_static!{
    static ref MONITOR: RwLock<Monitor> = RwLock::new(Monitor::new());
}

pub struct Monitor {
    information: AgentInformation,
    performance: Performance,
};

impl Monitor {
    fn new() -> Self {
        Self {
            information: Self::system_info(),
            performance: Self::performance(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MONITOR.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MONITOR.write().await
    }

    fn system_info() -> AgentInformation {
        let sys = System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        let cpu = sys.cpus().get(0).map(|cpu| cpu.name())
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
        let gpu_name = String::from_utf8_lossy(&gpu_name.stdout).to_string();
        Ok(gpu_name)
    }

    fn get_vram_total() -> Result<usize, String> {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=memory.total")
            .arg("--format=csv,noheader")
            .output()
            .map_err(|_| "Monitor: Fail to get gpu information.".to_string())?;
        let output_string = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = output_string.split_whitespace().collect();
        let vram_total = parts.get(0).ok_or("Monitor: Fail to parse gpu information.".to_string())?;
        let vram_total = vram_total.parse::<usize>().map_err(|_| "Monitor: Fail to parse gpu information.".to_string())?;
        Ok(vram_total)
    }

    pub fn performance() -> Performance {

        Performance {
            cpu: 0.0,
            ram: 0.0,
            gpu: 0.0,
            vram: 0.0,
        }
    }
}
