use async_trait::async_trait;
use thiserror::Error;
use tracing::warn;

pub use crate::models::Provider;

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
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[async_trait]
pub trait MetalProvider: Send + Sync {
    async fn create_server(&self, spec: &crate::models::ServerSpec) -> Result<crate::models::Server, ProviderError>;
    async fn get_server(&self, id: &str) -> Result<crate::models::Server, ProviderError>;
    async fn delete_server(&self, id: &str) -> Result<(), ProviderError>;
    async fn list_servers(&self) -> Result<Vec<crate::models::Server>, ProviderError>;
    async fn start_server(&self, id: &str) -> Result<(), ProviderError>;
    async fn stop_server(&self, id: &str) -> Result<(), ProviderError>;
    
    /// Check if server creation would succeed without actually creating it
    async fn validate_server_creation(&self, spec: &crate::models::ServerSpec) -> Result<(), ProviderError> {
        // Default implementation just tries to create and delete
        // Providers should override this with more efficient implementations
        let server = self.create_server(spec).await?;
        self.delete_server(&server.id).await.map_err(|e| {
            // Log the error but don't fail validation since we successfully created
            warn!("Failed to cleanup test server {}: {}", server.id, e);
            ProviderError::InvalidConfig("Failed to cleanup test server".to_string())
        })?;
        Ok(())
    }
}

pub mod cherry;
pub mod latitude;
pub mod ovh;

pub use cherry::CherryProvider;
pub use latitude::LatitudeProvider;
pub use ovh::OvhProvider;
