use super::{MetalProvider, ProviderError};
use crate::models::{Server, ServerSpec, ServerSpecs, ServerStatus};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::*;

const OVH_API_BASE: &str = "https://api.ovh.com/1.0";

pub struct OvhProvider {
    client: Client,
    endpoint: String,
    app_key: String,
    app_secret: String,
    consumer_key: String,
}

#[derive(Debug, Deserialize)]
struct OvhServer {
    #[serde(default)]
    server_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ip: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    datacenter: Option<String>,
}

#[derive(Serialize)]
struct OvhServerCreateRequest {
    #[serde(rename = "templateCode")]
    template_code: String,
    hostname: String,
    #[serde(rename = "datacenterId")]
    datacenter_id: i32,
}

impl OvhProvider {
    pub fn new(endpoint: String, app_key: String, app_secret: String, consumer_key: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            app_key,
            app_secret,
            consumer_key,
        }
    }

    // OVH uses timestamp-based signature
    fn signature(&self, method: &str, url: &str, body: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let to_sign = format!(
            "{}+{}+{}+{}{}{}",
            self.app_secret, timestamp, self.consumer_key, method, url, body
        );
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        to_sign.hash(&mut hasher);
        format!("$1${}", hasher.finish())
    }
}

#[async_trait]
impl MetalProvider for OvhProvider {
    async fn create_server(&self, spec: &ServerSpec) -> Result<Server, ProviderError> {
        info!("Creating server on OVH: {}", spec.name);

        // OVH dedicated server creation is complex - this is a simplified version
        let url = format!("{}/dedicated/server", OVH_API_BASE);
        
        let request = OvhServerCreateRequest {
            template_code: "ubuntu22.04".to_string(), // simplified
            hostname: spec.name.clone(),
            datacenter_id: 0, // would need to lookup
        };

        let body = serde_json::to_string(&request).unwrap_or_default();
        let sig = self.signature("POST", &url, &body);

        let response = self.client
            .post(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
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

        // Parse response - OVH returns task ID for async creation
        let task_id: String = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        Ok(Server {
            id: task_id,
            ip_address: String::new(),
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
        info!("Getting server from OVH: {}", id);

        let url = format!("{}/dedicated/server/{}", OVH_API_BASE, id);
        
        let sig = self.signature("GET", &url, "");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();

        let response = self.client
            .get(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &timestamp.to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Err(ProviderError::NotFound(id.to_string()));
        }

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        let server: OvhServer = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        Ok(Server {
            id: id.to_string(),
            ip_address: server.ip.unwrap_or_default(),
            hostname: server.server_name.or(server.name).unwrap_or_default(),
            status: match server.state.as_deref() {
                Some("ok") => ServerStatus::Active,
                Some("error") => ServerStatus::Error,
                _ => ServerStatus::Provisioning,
            },
            region: server.datacenter.unwrap_or_default(),
            specs: ServerSpecs {
                cpu_cores: 32,
                memory_gb: 64,
                storage_gb: 1000,
            },
        })
    }

    async fn delete_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Deleting server from OVH: {}", id);

        let url = format!("{}/dedicated/server/{}", OVH_API_BASE, id);
        let sig = self.signature("DELETE", &url, "");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();

        let response = self.client
            .delete(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &timestamp.to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
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
        info!("Listing servers from OVH");

        let url = format!("{}/dedicated/server", OVH_API_BASE);
        let sig = self.signature("GET", &url, "");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();

        let response = self.client
            .get(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &timestamp.to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        let server_names: Vec<String> = response.json().await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        // For each server name, get details
        let mut servers = Vec::new();
        for name in server_names {
            if let Ok(server) = self.get_server(&name).await {
                servers.push(server);
            }
        }

        Ok(servers)
    }

    async fn start_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Starting server on OVH: {}", id);

        let url = format!("{}/dedicated/server/{}/reboot", OVH_API_BASE, id);
        let sig = self.signature("POST", &url, "");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();

        let response = self.client
            .post(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &timestamp.to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        Ok(())
    }

    async fn stop_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Stopping server on OVH: {}", id);

        let url = format!("{}/dedicated/server/{}/shutdown", OVH_API_BASE, id);
        let sig = self.signature("POST", &url, "");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();

        let response = self.client
            .post(&url)
            .header("X-Ovh-Application", &self.app_key)
            .header("X-Ovh-Timestamp", &timestamp.to_string())
            .header("X-Ovh-Signature", &sig)
            .header("X-Ovh-Consumer", &self.consumer_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Status: {}", response.status())));
        }

        Ok(())
    }
}
