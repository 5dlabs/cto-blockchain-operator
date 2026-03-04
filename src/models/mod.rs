use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Cherry,
    Latitude,
    Ovh,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Cherry
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Cherry => write!(f, "cherry"),
            Provider::Latitude => write!(f, "latitude"),
            Provider::Ovh => write!(f, "ovh"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeStatus {
    pub phase: NodePhase,
    pub slot_height: Option<i64>,
    pub healthy: Option<bool>,
    pub slots_behind: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NodePhase {
    Pending,
    Initializing,
    Running,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Server {
    pub id: String,
    pub ip_address: String,
    pub hostname: String,
    pub status: ServerStatus,
    pub region: String,
    pub specs: ServerSpecs,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerStatus {
    Active,
    Inactive,
    Provisioning,
    Error,
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerStatus::Active => write!(f, "Active"),
            ServerStatus::Inactive => write!(f, "Inactive"),
            ServerStatus::Provisioning => write!(f, "Provisioning"),
            ServerStatus::Error => write!(f, "Error"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerSpecs {
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_gb: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerSpec {
    pub name: String,
    pub region: String,
    pub plan: String,
    pub image: String,
    pub ssh_keys: Vec<String>,
}
