use anyhow::Result;
use futures::StreamExt;
use kube::runtime::controller::Action;
use kube::runtime::finalizer;
use kube::runtime::Controller;
use kube::runtime::watcher::Config;
use kube::{Api, Client, ResourceExt};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

use crds::PolygonNode;
use crds::SolanaNode;

mod controllers;
mod crds;
mod models;
mod providers;

const FINALIZER_NAME: &str = "solananodes.blockchain.5dlabs.io";
const POLYGON_FINALIZER_NAME: &str = "polygonnodes.blockchain.5dlabs.io";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    info!("Starting CTO Blockchain Operator");

    // Create API watchers for different blockchain types
    let solana_nodes: Api<SolanaNode> = Api::default_namespaced(client.clone());
    let polygon_nodes: Api<PolygonNode> = Api::default_namespaced(client.clone());

    // Run both controllers concurrently
    let solana_future = solana_controller::run(solana_nodes);
    let polygon_future = polygon_controller::run(polygon_nodes);

    futures::future::join(solana_future, polygon_future).await;

    Ok(())
}

mod solana_controller {
    use super::*;

    pub async fn run(solana_nodes: Api<SolanaNode>) {
        Controller::new(solana_nodes, Config::default())
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
        let client = Client::try_default().await.map_err(Error::Kubernetes)?;
        let ns = solana_node.namespace().unwrap_or_else(|| "default".to_string());
        let name = solana_node.name_any();
        let solana_nodes_api: Api<SolanaNode> = Api::namespaced(client.clone(), &ns);

        info!("Reconciling SolanaNode: {}", name);

        finalizer(
            &solana_nodes_api,
            FINALIZER_NAME,
            solana_node,
            |event| async {
                match event {
                    finalizer::Event::Apply(sn) => apply((*sn).clone(), client).await,
                    finalizer::Event::Cleanup(sn) => cleanup((*sn).clone(), client).await,
                }
            },
        )
        .await
        .map_err(|e| Error::FinalizerError(Box::new(e)))
    }

    async fn apply(solana_node: SolanaNode, _client: Client) -> Result<Action, Error> {
        info!("Applying SolanaNode: {}", solana_node.name_any());
        Ok(Action::requeue(Duration::from_secs(300)))
    }

    async fn cleanup(solana_node: SolanaNode, _client: Client) -> Result<Action, Error> {
        info!("Cleaning up SolanaNode: {}", solana_node.name_any());
        Ok(Action::await_change())
    }

    fn error_policy(_solana_node: Arc<SolanaNode>, _error: &Error, _ctx: Arc<()>) -> Action {
        error!("Reconcile error: {:?}", _error);
        Action::requeue(Duration::from_secs(5))
    }
}

mod polygon_controller {
    use super::*;
    use crate::controllers::polygon::PolygonController;

    pub async fn run(polygon_nodes: Api<PolygonNode>) {
        Controller::new(polygon_nodes, Config::default())
            .run(reconcile, error_policy, Arc::new(()))
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("Reconciled Polygon {:?}", o),
                    Err(e) => warn!("Failed to reconcile Polygon: {}", e),
                }
            })
            .await;
    }

    async fn reconcile(
        polygon_node: Arc<PolygonNode>,
        _ctx: Arc<()>,
    ) -> Result<Action, Error> {
        let client = Client::try_default().await.map_err(Error::Kubernetes)?;
        let ns = polygon_node
            .namespace()
            .unwrap_or_else(|| "default".to_string());
        let name = polygon_node.name_any();
        let polygon_nodes_api: Api<PolygonNode> = Api::namespaced(client.clone(), &ns);

        info!("Reconciling PolygonNode: {}", name);

        finalizer(
            &polygon_nodes_api,
            POLYGON_FINALIZER_NAME,
            polygon_node,
            |event| async {
                match event {
                    finalizer::Event::Apply(pn) => apply((*pn).clone(), client).await,
                    finalizer::Event::Cleanup(pn) => cleanup((*pn).clone(), client).await,
                }
            },
        )
        .await
        .map_err(|e| Error::FinalizerError(Box::new(e)))
    }

    async fn apply(polygon_node: PolygonNode, client: Client) -> Result<Action, Error> {
        info!("Applying PolygonNode: {}", polygon_node.name_any());

        let controller = PolygonController::new(client);
        match controller.reconcile(&polygon_node).await {
            Ok(status) => {
                let requeue_secs = match status.phase {
                    Some(crate::crds::polygon::PolygonNodePhase::Running) => 300,
                    Some(crate::crds::polygon::PolygonNodePhase::Error) => 30,
                    _ => 60,
                };
                Ok(Action::requeue(Duration::from_secs(requeue_secs)))
            }
            Err(e) => {
                error!("Failed to reconcile PolygonNode: {}", e);
                Ok(Action::requeue(Duration::from_secs(30)))
            }
        }
    }

    async fn cleanup(polygon_node: PolygonNode, _client: Client) -> Result<Action, Error> {
        info!("Cleaning up PolygonNode: {}", polygon_node.name_any());
        Ok(Action::await_change())
    }

    fn error_policy(
        _polygon_node: Arc<PolygonNode>,
        _error: &Error,
        _ctx: Arc<()>,
    ) -> Action {
        error!("Polygon reconcile error: {:?}", _error);
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
    FinalizerError(#[source] Box<kube::runtime::finalizer::Error<Error>>),
}
