use super::{MetalProvider, ProviderError};
use crate::models::{Server, ServerSpec, ServerSpecs, ServerStatus};
use async_trait::async_trait;
use reqwest::header::ACCEPT;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::*;

const CHERRY_API_BASE: &str = "https://api.cherryservers.com";
const CREATE_SERVER_FIELDS: &str =
    "server,href,specs,plan,pricing,region,software,vlan,storage,bgp,id";
const GET_SERVER_FIELDS: &str =
    "server,href,specs,plan,pricing,ip,region,bmc,software,vlan,storage,bgp,id,name,os_raid_level,os_disk";
const LIST_SERVER_FIELDS: &str =
    "server,href,specs,plan,pricing,ip,region,software,vlan,storage,bgp,id,name";
const ACTION_SERVER_FIELDS: &str =
    "server,href,specs,plan,pricing,ip,region,bmc,software,vlan,storage,bgp,id,name";

pub struct CherryProvider {
    client: Client,
    api_key: String,
    team_id: String,
    project_id: String,
}

#[derive(Debug, Deserialize)]
struct CherryRegion {
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CherryServerResponse {
    id: i64,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ip: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    region: Option<CherryRegion>,
    #[serde(default)]
    specs: Option<Value>,
}

#[derive(Debug, Serialize)]
struct CherryDeployServerRequest {
    plan: String,
    image: String,
    region: String,
    hostname: String,
    ssh_keys: Vec<i64>,
}

#[derive(Debug, Serialize)]
struct CherryServerActionRequest<'a> {
    #[serde(rename = "type")]
    action_type: &'a str,
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

    fn endpoint(&self, path: &str) -> String {
        format!("{CHERRY_API_BASE}{path}")
    }

    fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .bearer_auth(&self.api_key)
            .header(ACCEPT, "application/json")
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        builder: RequestBuilder,
    ) -> Result<T, ProviderError> {
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !status.is_success() {
            return Err(Self::map_error(status, &body));
        }

        serde_json::from_str::<T>(&body).map_err(|e| {
            ProviderError::ApiError(format!(
                "Failed to decode Cherry response: {e}; body={body}"
            ))
        })
    }

    async fn send_no_content(&self, builder: RequestBuilder) -> Result<(), ProviderError> {
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if status.is_success() {
            Ok(())
        } else {
            Err(Self::map_error(status, &body))
        }
    }

    fn map_error(status: StatusCode, body: &str) -> ProviderError {
        let msg = if body.trim().is_empty() {
            format!("HTTP {}", status.as_u16())
        } else {
            body.to_string()
        };

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::AuthError(msg),
            StatusCode::NOT_FOUND => ProviderError::NotFound(msg),
            _ => ProviderError::ApiError(msg),
        }
    }

    fn status_from_cherry(state: Option<&str>, status: Option<&str>) -> ServerStatus {
        let state = state.unwrap_or_default().to_ascii_lowercase();
        let status = status.unwrap_or_default().to_ascii_lowercase();

        if state.contains("error")
            || state.contains("failed")
            || status.contains("error")
            || status.contains("failed")
        {
            return ServerStatus::Error;
        }

        if state.contains("inactive")
            || state.contains("stopped")
            || status.contains("power-off")
            || status.contains("powered off")
        {
            return ServerStatus::Inactive;
        }

        if state.contains("active") && !status.contains("deploy") {
            return ServerStatus::Active;
        }

        if status.contains("provision")
            || status.contains("deploy")
            || status.contains("build")
            || status.contains("queue")
            || status.contains("install")
        {
            return ServerStatus::Provisioning;
        }

        ServerStatus::Provisioning
    }

    fn int_from_specs(specs: &Value, keys: &[&str]) -> i32 {
        for key in keys {
            if let Some(v) = specs.get(key).and_then(Value::as_i64) {
                return v as i32;
            }
            if let Some(v) = specs.get(key).and_then(Value::as_u64) {
                return v as i32;
            }
            if let Some(v) = specs
                .get(key)
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<i32>().ok())
            {
                return v;
            }
        }
        0
    }

    fn map_server_specs(specs: Option<&Value>) -> ServerSpecs {
        if let Some(specs) = specs {
            return ServerSpecs {
                cpu_cores: Self::int_from_specs(
                    specs,
                    &["cpu_cores", "cpu", "cores", "cpus", "cpu_count"],
                ),
                memory_gb: Self::int_from_specs(specs, &["memory_gb", "memory", "ram", "ram_gb"]),
                storage_gb: Self::int_from_specs(
                    specs,
                    &["storage_gb", "storage", "disk", "disk_gb"],
                ),
            };
        }

        ServerSpecs {
            cpu_cores: 0,
            memory_gb: 0,
            storage_gb: 0,
        }
    }

    fn map_server(
        &self,
        source: CherryServerResponse,
        fallback_region: Option<&str>,
        fallback_hostname: Option<&str>,
    ) -> Server {
        let region = source
            .region
            .as_ref()
            .and_then(|r| r.slug.clone().or_else(|| r.name.clone()))
            .or_else(|| fallback_region.map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());

        let hostname = source
            .hostname
            .clone()
            .or(source.name.clone())
            .or_else(|| fallback_hostname.map(str::to_string))
            .unwrap_or_default();

        Server {
            id: source.id.to_string(),
            ip_address: source.ip.unwrap_or_default(),
            hostname,
            status: Self::status_from_cherry(source.state.as_deref(), source.status.as_deref()),
            region,
            specs: Self::map_server_specs(source.specs.as_ref()),
        }
    }
}

#[async_trait]
impl MetalProvider for CherryProvider {
    async fn create_server(&self, spec: &ServerSpec) -> Result<Server, ProviderError> {
        info!(
            "Creating server on Cherry: {} (team_id={}, project_id={})",
            spec.name, self.team_id, self.project_id
        );

        let ssh_keys = spec
            .ssh_keys
            .iter()
            .filter_map(|key| key.parse::<i64>().ok())
            .collect::<Vec<_>>();
        let payload = CherryDeployServerRequest {
            plan: spec.plan.clone(),
            image: spec.image.clone(),
            region: spec.region.clone(),
            hostname: spec.name.clone(),
            ssh_keys,
        };

        let url = self.endpoint(&format!("/v1/projects/{}/servers", self.project_id));
        let created: CherryServerResponse = self
            .send_json(
                self.request(self.client.post(url))
                    .query(&[("fields", CREATE_SERVER_FIELDS)])
                    .json(&payload),
            )
            .await?;

        Ok(self.map_server(created, Some(&spec.region), Some(&spec.name)))
    }

    async fn get_server(&self, id: &str) -> Result<Server, ProviderError> {
        info!("Getting server from Cherry: {}", id);

        let url = self.endpoint(&format!("/v1/servers/{id}"));
        let server: CherryServerResponse = self
            .send_json(
                self.request(self.client.get(url))
                    .query(&[("fields", GET_SERVER_FIELDS)]),
            )
            .await?;

        Ok(self.map_server(server, None, None))
    }

    async fn delete_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Deleting server from Cherry: {}", id);

        let url = self.endpoint(&format!("/v1/servers/{id}"));
        self.send_no_content(self.request(self.client.delete(url)))
            .await
    }

    async fn list_servers(&self) -> Result<Vec<Server>, ProviderError> {
        info!(
            "Listing servers from Cherry (project_id={})",
            self.project_id
        );

        let url = self.endpoint(&format!("/v1/projects/{}/servers", self.project_id));
        let servers: Vec<CherryServerResponse> = self
            .send_json(self.request(self.client.get(url)).query(&[
                ("fields", LIST_SERVER_FIELDS),
                ("limit", "100"),
                ("offset", "0"),
            ]))
            .await?;

        Ok(servers
            .into_iter()
            .map(|s| self.map_server(s, None, None))
            .collect())
    }

    async fn start_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Starting server on Cherry: {}", id);

        let url = self.endpoint(&format!("/v1/servers/{id}/actions"));
        let payload = CherryServerActionRequest {
            action_type: "power-on",
        };

        let _: Value = self
            .send_json(
                self.request(self.client.post(url))
                    .query(&[("fields", ACTION_SERVER_FIELDS)])
                    .json(&payload),
            )
            .await?;
        Ok(())
    }

    async fn stop_server(&self, id: &str) -> Result<(), ProviderError> {
        info!("Stopping server on Cherry: {}", id);

        let url = self.endpoint(&format!("/v1/servers/{id}/actions"));
        let payload = CherryServerActionRequest {
            action_type: "power-off",
        };

        let _: Value = self
            .send_json(
                self.request(self.client.post(url))
                    .query(&[("fields", ACTION_SERVER_FIELDS)])
                    .json(&payload),
            )
            .await?;
        Ok(())
    }
}
