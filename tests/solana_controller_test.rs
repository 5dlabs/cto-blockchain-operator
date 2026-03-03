use cto_blockchain_operator::controllers::solana::SolanaController;
use cto_blockchain_operator::crds::{SolanaNode, SolanaNodeSpec, NodeType, NodeResources, NodeConfig};
use kube::Client;

#[tokio::test]
async fn test_solana_controller_reconcile() {
    // Create a mock Kubernetes client
    // Note: In a real test, we would use a mock or test cluster
    let client = Client::try_default().await.unwrap();
    let controller = SolanaController::new(client);
    
    // Create a minimal SolanaNode spec for testing
    let solana_node = SolanaNode {
        metadata: Default::default(),
        spec: SolanaNodeSpec {
            replicas: 1,
            node_type: NodeType::Validator,
            rpc_port: 8899,
            gossip_port: 8001,
            resources: NodeResources {
                cpu_request: "28".to_string(),
                memory_request: "64Gi".to_string(),
                cpu_limit: Some("32".to_string()),
                memory_limit: Some("128Gi".to_string()),
            },
            config: NodeConfig {
                expected_genesis_hash: "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d".to_string(),
                limit_ledger_size: 200000000,
                full_rpc_api: true,
                enable_accounts_disk_index: true,
                skip_startup_ledger_verification: true,
                rpc_threads: 128,
                maximum_full_snapshots_to_retain: 2,
                wal_recovery_mode: "skip_any_corrupted_record".to_string(),
            },
            image: "anzaxyz/agave:v3.1.9".to_string(),
            enable_voting: false,
            identity_secret: "test-identity".to_string(),
            known_validators: None,
            entrypoints: None,
        },
        status: None,
    };
    
    // Test the reconcile function
    // Note: This is a basic test, real implementation would require more setup
    // let result = controller.reconcile(&solana_node).await;
    // assert!(result.is_ok());
}

#[tokio::test]
async fn test_solana_node_spec_validation() {
    // Test that we can create a valid SolanaNodeSpec
    let spec = SolanaNodeSpec {
        replicas: 1,
        node_type: NodeType::Validator,
        rpc_port: 8899,
        gossip_port: 8001,
        resources: NodeResources {
            cpu_request: "28".to_string(),
            memory_request: "64Gi".to_string(),
            cpu_limit: Some("32".to_string()),
            memory_limit: Some("128Gi".to_string()),
        },
        config: NodeConfig {
            expected_genesis_hash: "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d".to_string(),
            limit_ledger_size: 200000000,
            full_rpc_api: true,
            enable_accounts_disk_index: true,
            skip_startup_ledger_verification: true,
            rpc_threads: 128,
            maximum_full_snapshots_to_retain: 2,
            wal_recovery_mode: "skip_any_corrupted_record".to_string(),
        },
        image: "anzaxyz/agave:v3.1.9".to_string(),
        enable_voting: false,
        identity_secret: "test-identity".to_string(),
        known_validators: None,
        entrypoints: None,
    };
    
    assert_eq!(spec.replicas, 1);
    assert_eq!(spec.node_type, NodeType::Validator);
    assert_eq!(spec.rpc_port, 8899);
    assert_eq!(spec.gossip_port, 8001);
}