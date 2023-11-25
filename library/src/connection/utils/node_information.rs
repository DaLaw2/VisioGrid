use std::fmt;
use std::str::FromStr;
use std::string::ToString;

#[derive(Clone)]
pub struct NodeInformation {
    pub device_name: String,
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub ram: usize,
    pub gpu: String,
    pub gram: usize,
}

impl ToString for NodeInformation {
    fn to_string(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.device_name,
            self.os,
            self.cpu,
            self.cores,
            self.ram,
            self.gpu,
            self.gram
        )
    }
}

impl FromStr for NodeInformation {
    type Err = fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 7 {
            return Err(fmt::Error);
        }

        Ok(NodeInformation {
            device_name: parts[0].to_string(),
            os: parts[1].to_string(),
            cpu: parts[2].to_string(),
            cores: parts[3].parse().map_err(|_| fmt::Error)?,
            ram: parts[4].parse().map_err(|_| fmt::Error)?,
            gpu: parts[5].to_string(),
            gram: parts[6].parse().map_err(|_| fmt::Error)?,
        })
    }
}
