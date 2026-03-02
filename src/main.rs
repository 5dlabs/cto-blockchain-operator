use anyhow::Result;
use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType, Recorder, Reporter};
use kube::runtime::finalizer;
use kube::{Api, Client, ResourceExt};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

use crds::SolanaNode;

mod crds;
mod controllers;
mod models;
mod providers;

const FINALIZER_NAME: &str = "solananodes.blockchain.5dlabs.io";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    
    info!("Starting CTO Blockchain Operator");
    
    // Create controllers for different blockchain types
    let solana_nodes: Api<SolanaNode> = Api::default_namespaced(client.clone());
    
    // Start controller
    controller::run(solana_nodes).await;
    
    Ok(())
}

mod controller {
    use super::*;
    
    pub async fn run(solana_nodes: Api<SolanaNode>) {
        kube::runtime::controller::Builder::new(solana_nodes, Default::default())
            .run(reconcile, error_policy, Arc::new(()))
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("Reconciled {:?}", o),
                    Err(e) => warn!("Failed to reconcile: {}", e),
                }
            })
            .await;
    }

    async fn reconcile(solana_node: Arc<SolanaNode>, _ctx: Arc<()>) -> Result<Action, Error> {
        let mut solana_node = (*solana_node).clone();
        let client = kube::client::Client::try_default().await?;
        let ns = solana_node.namespace().unwrap();
        let name = solana_node.name_any();
        let solana_nodes_api: Api<SolanaNode> = Api::namespaced(client.clone(), &ns);
        
        info!("Reconciling SolanaNode: {}", name);
        
        // Finalizer handling
        finalizer(&solana_nodes_api, FINALIZER_NAME, solana_node, |event| async {
            match event {
                finalizer::Event::Apply(sn) => {
                    apply(solana_node, client).await
                }
                finalizer::Event::Cleanup(sn) => {
                    cleanup(solana_node, client).await
                }
            }
        }).await.map_err(Error::FinalizerError)
    }
    
    async fn apply(solana_node: SolanaNode, client: Client) -> Result<Action, Error> {
        info!("Applying SolanaNode: {}", solana_node.name_any());
        // Actual operator logic goes here
        Ok(Action::requeue(Duration::from_secs(300)))
    }
    
    async fn cleanup(solana_node: SolanaNode, client: Client) -> Result<Action, Error> {
        info!("Cleaning up SolanaNode: {}", solana_node.name_any());
        // Cleanup logic goes here
        Ok(Action::await_change())
    }
    
    fn error_policy(_solana_node: Arc<SolanaNode>, _error: &Error, _ctx: Arc<()>) -> Action {
        error!("Reconcile error: {:?}", _error);
        Action::requeue(Duration::from_secs(5))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serialization error: {0}")]
    Serialization(#[source] serde_json::Error),
    #[error("Kubernetes API error: {0}")]
    Kubernetes(#[source] kube::Error),
    #[error("Finalizer error: {0}")]
    FinalizerError(#[source] finalizer::Error),
}
