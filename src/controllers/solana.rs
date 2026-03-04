use crate::crds::SolanaNode;
use crate::providers::Provider;
use crate::models::{NodePhase, NodeStatus, ServerSpec};
use crate::providers::{CherryProvider, LatitudeProvider, MetalProvider, OvhProvider};
use kube::{Api, Client, ResourceExt};
use std::sync::Arc;
use thiserror::Error;
use tracing::*;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("Failed to provision server: {0}")]
    ProvisionError(String),
    #[error("Failed to create Kubernetes resources: {0}")]
    K8sError(String),
    #[error("Node in error state: {0}")]
    NodeError(String),
    #[error("Kubernetes API error: {0}")]
    Kubernetes(#[from] kube::Error),
}

pub struct SolanaController {
    client: Client,
}

impl SolanaController {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn reconcile(&self, crd: &SolanaNode) -> Result<NodeStatus, ControllerError> {
        let name = crd.name_any();
        let namespace = crd.namespace().unwrap_or("default".to_string());
        info!(
            "Reconciling SolanaNode {} in namespace {}",
            name, namespace
        );

        // Phase 1: Provision the server
        let provider: Arc<dyn MetalProvider> = match crd.spec.provider {
            Provider::Cherry => {
                let api_key = std::env::var("CHERRY_API_KEY")
                    .unwrap_or_else(|_| "test-key".to_string());
                let team_id = std::env::var("CHERRY_TEAM_ID")
                    .unwrap_or_else(|_| "190658".to_string());
                let project_id = std::env::var("CHERRY_PROJECT_ID")
                    .unwrap_or_else(|_| "264136".to_string());
                Arc::new(CherryProvider::new(api_key, team_id, project_id))
            }
            Provider::Latitude => {
                let api_key = std::env::var("LATITUDE_API_KEY")
                    .unwrap_or_else(|_| "test-key".to_string());
                Arc::new(LatitudeProvider::new(api_key))
            }
            Provider::Ovh => {
                let endpoint = std::env::var("OVH_ENDPOINT")
                    .unwrap_or_else(|_| "ovh-us".to_string());
                let app_key = std::env::var("OVH_APP_KEY")
                    .unwrap_or_else(|_| "test-key".to_string());
                let app_secret = std::env::var("OVH_APP_SECRET")
                    .unwrap_or_else(|_| "test-secret".to_string());
                let consumer_key = std::env::var("OVH_CONSUMER_KEY")
                    .unwrap_or_else(|_| "test-consumer".to_string());
                Arc::new(OvhProvider::new(endpoint, app_key, app_secret, consumer_key))
            }
        };

        let server_spec = ServerSpec {
            name: name.clone(),
            region: crd.spec.region.clone(),
            plan: "c3.large.arm".to_string(),
            image: "ubuntu_22_04".to_string(),
            ssh_keys: vec![],
        };

        match provider.create_server(&server_spec).await {
            Ok(server) => {
                info!("Successfully provisioned server: {:?}", server);
                // Phase 2: Create Kubernetes resources (StatefulSet, Service, etc.)
                // This would be the next step
                Ok(NodeStatus {
                    phase: Some(NodePhase::Initializing),
                    slot_height: None,
                    healthy: None,
                    slots_behind: None,
                })
            }
            Err(e) => {
                error!("Failed to provision server: {}", e);
                Err(ControllerError::ProvisionError(e.to_string()))
            }
        }
    }
}

// Helper functions for Kubernetes resource management
async fn create_statefulset(
    client: &Client,
    solana_node: &SolanaNode,
) -> Result<(), ControllerError> {
    // Implementation would create a StatefulSet for the Solana node
    info!(
        "Creating StatefulSet for Solana node: {}",
        solana_node.name_any()
    );
    Ok(())
}

async fn create_service(client: &Client, solana_node: &SolanaNode) -> Result<(), ControllerError> {
    // Implementation would create a Service for the Solana node
    info!(
        "Creating Service for Solana node: {}",
        solana_node.name_any()
    );
    Ok(())
}
