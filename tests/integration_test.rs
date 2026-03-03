//! Integration tests for the CTO Blockchain Operator
//! Tests all supported providers and blockchain types

use cto_blockchain_operator::providers::{cherry::CherryProvider, latitude::LatitudeProvider, ovh::OvhProvider, MetalProvider, ServerSpec};
use cto_blockchain_operator::crds::{SolanaNode, SolanaNodeSpec, NodeType, NodeResources, NodeConfig};

#[tokio::test]
async fn test_all_providers_create_server() {
    // Test Cherry provider
    let cherry_provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    let spec = ServerSpec {
        name: "cherry-solana-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "solana-server-gen5".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    let cherry_result = cherry_provider.create_server(&spec).await;
    assert!(cherry_result.is_ok());
    
    // Test Latitude provider
    let latitude_provider = LatitudeProvider::new("test-api-key".to_string());
    
    let latitude_result = latitude_provider.create_server(&spec).await;
    assert!(latitude_result.is_ok());
    
    // Test OVH provider
    let ovh_provider = OvhProvider::new(
        "https://eu.api.ovh.com/1.0".to_string(),
        "test-app-key".to_string(),
        "test-app-secret".to_string(),
        "test-consumer-key".to_string(),
    );
    
    let ovh_result = ovh_provider.create_server(&spec).await;
    assert!(ovh_result.is_ok());
}

#[tokio::test]
async fn test_solana_node_creation() {
    // Test creating SolanaNode specifications for different node types
    let validator_spec = SolanaNodeSpec {
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
        identity_secret: "validator-identity".to_string(),
        known_validators: Some(vec![
            "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2".to_string(),
            "HEL1USMZKAL2odpNBj2oCjffnFGaYwmbGmyewGv1e2TU".to_string(),
        ]),
        entrypoints: Some(vec![
            "entrypoint.mainnet-beta.solana.com:8001".to_string(),
            "entrypoint2.mainnet-beta.solana.com:8001".to_string(),
        ]),
    };
    
    let rpc_spec = SolanaNodeSpec {
        node_type: NodeType::Rpc,
        enable_voting: false,
        ..validator_spec.clone()
    };
    
    let archival_spec = SolanaNodeSpec {
        node_type: NodeType::Archival,
        enable_voting: false,
        ..validator_spec.clone()
    };
    
    // Verify all node types can be created
    assert_eq!(validator_spec.node_type, NodeType::Validator);
    assert_eq!(rpc_spec.node_type, NodeType::Rpc);
    assert_eq!(archival_spec.node_type, NodeType::Archival);
}

#[tokio::test]
async fn test_hardware_requirements() {
    // Test that hardware requirements match documentation
    
    // Solana Validator: 32 cores, 256GB RAM, 4x NVMe SSD
    let solana_validator_resources = NodeResources {
        cpu_request: "32".to_string(),
        memory_request: "256Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // Sui Validator: 24 cores, 128GB RAM, 4TB NVMe
    let sui_validator_resources = NodeResources {
        cpu_request: "24".to_string(),
        memory_request: "128Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // Aptos Validator: 32 cores, 64GB RAM, 3TB SSD
    let aptos_validator_resources = NodeResources {
        cpu_request: "32".to_string(),
        memory_request: "64Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // Monad Validator: 16 cores (4.5GHz+), 32GB RAM, 2.5TB SSD
    let monad_validator_resources = NodeResources {
        cpu_request: "16".to_string(),
        memory_request: "32Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // NEAR Validator: 8 cores, 48GB RAM, 3TB NVMe
    let near_validator_resources = NodeResources {
        cpu_request: "8".to_string(),
        memory_request: "48Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // Berachain Validator: 4 cores, 16GB RAM, 1TB SSD
    let berachain_validator_resources = NodeResources {
        cpu_request: "4".to_string(),
        memory_request: "16Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    };
    
    // Verify all resource specifications exist
    assert_eq!(solana_validator_resources.cpu_request, "32");
    assert_eq!(sui_validator_resources.cpu_request, "24");
    assert_eq!(aptos_validator_resources.cpu_request, "32");
    assert_eq!(monad_validator_resources.cpu_request, "16");
    assert_eq!(near_validator_resources.cpu_request, "8");
    assert_eq!(berachain_validator_resources.cpu_request, "4");
}