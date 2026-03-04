use super::{MetalProvider, ProviderError, Server, ServerSpec, ServerSpecs, ServerStatus};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::*;

const LATITUDE_API_BASE: &str = "https://api.latitude.sh/v1";

pub struct LatitudeProvider {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct LatitudeServer {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    ip_address: Option<String>,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    specs: Option<LatitudeSpecs>,
}

#[derive(Debug, Deserialize)]
struct LatitudeSpecs {
    #[serde(default)]
    cpus: Option<i32>,
    #[serde(default)]
    memory_gb: Option<i32>,
    #[serde(default)]
    storage_gb: Option<i32>,
}

#[derive(Serialize)]
struct LatitudeCreateRequest {
    name: String,
    region: String,
    plan: String,
    #[serde(rename = "os")]
    os: String,
    #[serde(default)]
    ssh_keys: Option<Vec<String>>,
}

impl LatitudeProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[async_trait]
impl MetalProvider for LatitudeProvider {
    async fn create_server(&self, spec: &ServerSpec) -> Result<Server, ProviderError> {
        info!("Creating server on Latitude: {}", spec.name);

        let url = format!("{}/servers", LATITUDE_API_BASE);
        
        let request = LatitudeCreateRequest {
            name: spec.name.clone(),
            region: spec.region.clone(),
            plan: spec.plan.clone(),
            os: spec.image.clone(),
            ssh_keys: if spec.ssh_keys.is_empty() { None } else { Some(spec.ssh_keys.clone()) },
        };

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("Status {}: {}", status, body)));
        }

        let server: LatitudeServer = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        Ok(Server {
            id: server.id,
            ip_address: server.ip_address.unwrap_or_default(),
            hostname: server.name,
            status: ServerStatus::Provisioning,
            region: server.region.unwrap_or(spec.region),
            specs: ServerSpecs {
                cpu_cores: server.specs.and_then(|s| s.cpus).unwrap_or(32),
                memory_gb: server.specs.and_then(|s| s.memory_gb).unwrap_or(64),
                storage_gb: server.specs.and_then(|s| s.storage_gb).unwrap_or(1000),
            },
        })
    }

    async fn get_server(&self, id: &str) -> Result<Server, ProviderError> {
        info!("Getting server from Latitude: {}", id);

        let url = format!("{}/servers/{}", LATITUDE_API_BASE, id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(ProviderError::NotFound(id.to_string()));
        }

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        let server: LatitudeServer = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        Ok(Server {
            id: server.id,
            ip_address: server.ip_address.unwrap_or_default(),
            hostname: server.name,
            status: match server.status.as_str() {
                "active" | "running" => ServerStatus::Active,
                "provisioning" => ServerStatus::Provisioning,
                "stopped" | "shutoff" => ServerStatus::Inactive,
                _ => ServerStatus::Provisioning,
            },
            region: server.region.unwrap_or_default(),
            specs: ServerSpecs {
                cpu_cores: server.specs.and_then(|s| s.cpus).unwrap_or(32),
                memory_gb: server.specs.and_then(|s| s.memory_gb).unwrap_or(64),
                storage_gb: server.specs.and_then(|s| s.storage_gb).unwrap_or(1000),
            },
        })
    }

    async fn delete_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Deleting server from Latitude: {}", id);

        let url = format!("{}/servers/{}", LATITUDE_API_BASE, id);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(ProviderError::NotFound(id.to_string()));
        }

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        Ok(())
    }

    async fn list_servers(&self) -> Result<Vec<Server>, ProviderError> {
        info!("Listing servers from Latitude");

        let url = format!("{}/servers", LATITUDE_API_BASE);
        
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        let servers: Vec<LatitudeServer> = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        Ok(servers.into_iter().map(|s| Server {
            id: s.id,
            ip_address: s.ip_address.unwrap_or_default(),
            hostname: s.name,
            status: match s.status.as_str() {
                "active" | "running" => ServerStatus::Active,
                "provisioning" => ServerStatus::Provisioning,
                "stopped" | "shutoff" => ServerStatus::Inactive,
                _ => ServerStatus::Provisioning,
            },
            region: s.region.unwrap_or_default(),
            specs: ServerSpecs {
                cpu_cores: s.specs.and_then(|sp| sp.cpus).unwrap_or(32),
                memory_gb: s.specs.and_then(|sp| sp.memory_gb).unwrap_or(64),
                storage_gb: s.specs.and_then(|sp| sp.storage_gb).unwrap_or(1000),
            },
        }).collect())
    }

    async fn start_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Starting server on Latitude: {}", id);

        let url = format!("{}/servers/{}/actions", LATITUDE_API_BASE, id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "action": "start" }))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        Ok(())
    }

    async fn stop_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Stopping server on Latitude: {}", id);

        let url = format!("{}/servers/{}/actions", LATITUDE_API_BASE, id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "action": "stop" }))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        Ok(())
    }
}
