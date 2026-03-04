use anyhow::Result;
use cto_blockchain_operator::controllers::solana::SolanaController;
use cto_blockchain_operator::crds::SolanaNode;
use futures::StreamExt;
use kube::runtime::controller::Action;
use kube::runtime::finalizer;
use kube::{Api, Client, ResourceExt};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

const FINALIZER_NAME: &str = "solananodes.blockchain.5dlabs.io";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    info!("Starting CTO Blockchain Operator");

    let solana_nodes: Api<SolanaNode> = Api::default_namespaced(client.clone());
    controller::run(solana_nodes).await;

    Ok(())
}

mod controller {
    use super::*;
    use kube::api::{Patch, PatchParams};

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
        let solana_node = (*solana_node).clone();
        let client = kube::client::Client::try_default().await?;
        let ns = solana_node
            .namespace()
            .unwrap_or_else(|| "default".to_string());
        let solana_nodes_api: Api<SolanaNode> = Api::namespaced(client.clone(), &ns);

        info!("Reconciling SolanaNode: {}", solana_node.name_any());

        finalizer(
            &solana_nodes_api,
            FINALIZER_NAME,
            solana_node,
            |event| async {
                match event {
                    finalizer::Event::Apply(sn) => apply(sn, client.clone()).await,
                    finalizer::Event::Cleanup(sn) => cleanup(sn, client.clone()).await,
                }
            },
        )
        .await
        .map_err(|e| Error::Finalizer(e.to_string()))
    }

    async fn apply(solana_node: SolanaNode, client: Client) -> Result<Action, Error> {
        info!("Applying SolanaNode: {}", solana_node.name_any());

        let ns = solana_node
            .namespace()
            .unwrap_or_else(|| "default".to_string());
        let name = solana_node.name_any();
        let solana_nodes_api: Api<SolanaNode> = Api::namespaced(client.clone(), &ns);

        let controller = SolanaController::new(client);
        let status = controller.reconcile(&solana_node).await?;

        let status_patch = serde_json::json!({
            "apiVersion": "blockchain.5dlabs.io/v1alpha1",
            "kind": "SolanaNode",
            "status": status,
        });

        solana_nodes_api
            .patch_status(
                &name,
                &PatchParams::apply("cto-blockchain-operator"),
                &Patch::Merge(&status_patch),
            )
            .await?;

        Ok(Action::requeue(Duration::from_secs(30)))
    }

    async fn cleanup(solana_node: SolanaNode, _client: Client) -> Result<Action, Error> {
        info!("Cleaning up SolanaNode: {}", solana_node.name_any());
        Ok(Action::await_change())
    }

    fn error_policy(_solana_node: Arc<SolanaNode>, error: &Error, _ctx: Arc<()>) -> Action {
        error!("Reconcile error: {:?}", error);
        Action::requeue(Duration::from_secs(5))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Kubernetes API error: {0}")]
    Kubernetes(#[from] kube::Error),
    #[error("Controller error: {0}")]
    Controller(#[from] cto_blockchain_operator::controllers::solana::ControllerError),
    #[error("Finalizer error: {0}")]
    Finalizer(String),
}
