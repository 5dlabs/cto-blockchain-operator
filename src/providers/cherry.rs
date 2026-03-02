use super::{MetalProvider, ProviderError, Server, ServerSpec, ServerSpecs, ServerStatus};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::*;

pub struct CherryProvider {
    client: Client,
    api_key: String,
    team_id: String,
    project_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CherryServer {
    id: i64,
    hostname: String,
    ip_addresses: Vec<CherryIp>,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CherryIp {
    address: String,
    #[serde(rename = "type")]
    ip_type: String,
}

impl CherryProvider {
    pub fn new(api_key: String, team_id: String, project_id: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            team_id,
            project_id,
        }
    }
}

#[async_trait]
impl MetalProvider for CherryProvider {
    async fn create_server(&self, spec: &ServerSpec) -> Result<Server, ProviderError> {
        info!("Creating server on Cherry: {}", spec.name);
        // Implementation would use Cherry API to create server
        Ok(Server {
            id: "cherry-12345".to_string(),
            ip_address: "192.168.1.100".to_string(),
            hostname: spec.name.clone(),
            status: ServerStatus::Provisioning,
            region: spec.region.clone(),
            specs: ServerSpecs {
                cpu_cores: 32,
                memory_gb: 64,
                storage_gb: 1000,
            },
        })
    }

    async fn get_server(&self, id: &str) -> Result<Server, ProviderError> {
        info!("Getting server from Cherry: {}", id);
        // Implementation would query Cherry API
        Ok(Server {
            id: id.to_string(),
            ip_address: "192.168.1.100".to_string(),
            hostname: "solana-node".to_string(),
            status: ServerStatus::Active,
            region: "nl-ams".to_string(),
            specs: ServerSpecs {
                cpu_cores: 32,
                memory_gb: 64,
                storage_gb: 1000,
            },
        })
    }

    async fn delete_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Deleting server from Cherry: {}", id);
        // Implementation would delete server via Cherry API
        Ok(())
    }

    async fn list_servers(&self) -> Result<Vec<Server>, ProviderError> {
        info!("Listing servers from Cherry");
        // Implementation would list servers from Cherry API
        Ok(vec![])
    }

    async fn start_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Starting server on Cherry: {}", id);
        // Implementation would start server via Cherry API
        Ok(())
    }

    async fn stop_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Stopping server on Cherry: {}", id);
        // Implementation would stop server via Cherry API
        Ok(())
    }
}
