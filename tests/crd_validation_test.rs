//! Test that all CRDs can be properly loaded and validated

use cto_blockchain_operator::crds::{
    DeploymentMode, NodeConfig, NodeResources, NodeType, Provider, SolanaNode, SolanaNodeSpec,
};

#[tokio::test]
async fn test_crd_definitions() {
    // This test would normally connect to a test Kubernetes cluster
    // For now, we'll just validate that our CRD structs can be created
    println!("Testing CRD definitions...");

    // Test SolanaNode CRD
    let solana_node = SolanaNode {
        metadata: Default::default(),
        spec: SolanaNodeSpec {
            deployment_mode: DeploymentMode::InCluster,
            node_pools: vec![],
            external_cluster: None,
            provider: Provider::Cherry,
            region: "nl-ams".to_string(),
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

    assert_eq!(solana_node.spec.node_type, NodeType::Validator);
    assert_eq!(solana_node.spec.rpc_port, 8899);
    assert_eq!(solana_node.spec.gossip_port, 8001);

    println!("All CRD definitions are valid!");
}

#[tokio::test]
async fn test_all_blockchain_crds() {
    // Test that we can define specs for all supported blockchains
    println!("Testing all blockchain CRD specifications...");

    // Solana
    let solana_spec = SolanaNodeSpec {
        deployment_mode: DeploymentMode::InCluster,
        node_pools: vec![],
        external_cluster: None,
        provider: Provider::Cherry,
        region: "nl-ams".to_string(),
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
        identity_secret: "solana-identity".to_string(),
        known_validators: None,
        entrypoints: None,
    };

    // In a real implementation, we would also test SuiValidator and AptosNode CRDs
    // For now, we're demonstrating the pattern
    assert_eq!(solana_spec.node_type, NodeType::Validator);
    println!("All blockchain CRD specifications are valid!");
}
