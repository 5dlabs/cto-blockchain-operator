use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("Server not found: {0}")]
    NotFound(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}

#[async_trait]
pub trait MetalProvider: Send + Sync {
    async fn create_server(&self, spec: &crate::models::ServerSpec) -> Result<crate::models::Server, ProviderError>;
    async fn get_server(&self, id: &str) -> Result<crate::models::Server, ProviderError>;
    async fn delete_server(&self, id: &str) -> Result<(), ProviderError>;
    async fn list_servers(&self) -> Result<Vec<crate::models::Server>, ProviderError>;
    async fn start_server(&self, id: &str) -> Result<(), ProviderError>;
    async fn stop_server(&self, id: &str) -> Result<(), ProviderError>;
}

pub mod cherry;
pub mod latitude;
pub mod ovh;

pub use cherry::CherryProvider;
pub use latitude::LatitudeProvider;
pub use ovh::OvhProvider;
