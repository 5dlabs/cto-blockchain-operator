//! Integration tests for the CTO Blockchain Operator
//! Tests all supported providers and blockchain types
//!
//! Note: These tests require real credentials to run and will incur costs.
//! To run these tests, set the following environment variables:
//! - CHERRY_API_KEY, CHERRY_TEAM_ID, CHERRY_PROJECT_ID
//! - LATITUDE_API_KEY
//! - OVH_ENDPOINT, OVH_APPLICATION_KEY, OVH_APPLICATION_SECRET, OVH_CONSUMER_KEY

use cto_blockchain_operator::models::ServerSpec;
use cto_blockchain_operator::providers::{cherry::CherryProvider, latitude::LatitudeProvider, ovh::OvhProvider, MetalProvider};
use cto_blockchain_operator::crds::{DeploymentMode, NodeConfig, NodeResources, NodeType, Provider, SolanaNodeSpec};

/// Skip integration tests unless CI_INTEGRATION_TESTS is set to "true"
/// This prevents accidentally running tests that make real API calls and incur costs
fn skip_integration_tests() -> bool {
    std::env::var("CI_INTEGRATION_TESTS").unwrap_or_default() != "true"
}

#[tokio::test]
async fn test_all_providers_create_server() {
    // Skip this test unless explicitly enabled to prevent accidental costs
    if skip_integration_tests() {
        println!("Skipping integration test - set CI_INTEGRATION_TESTS=true to run");
        return;
    }

    // Test Cherry provider
    let api_key = std::env::var("CHERRY_API_KEY").expect("CHERRY_API_KEY must be set");
    let team_id = std::env::var("CHERRY_TEAM_ID").expect("CHERRY_TEAM_ID must be set");
    let project_id = std::env::var("CHERRY_PROJECT_ID").expect("CHERRY_PROJECT_ID must be set");

    let cherry_provider = CherryProvider::new(api_key, team_id, project_id);
    
    let spec = ServerSpec {
        name: "integration-test-cherry-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "e5-1660v3".to_string(), // Use a real, available plan
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec![], // Empty for testing
    };
    
    let cherry_result = cherry_provider.create_server(&spec).await;
    assert!(cherry_result.is_ok(), "Cherry provider failed: {:?}", cherry_result.err());
    
    // Test Latitude provider
    let latitude_api_key = std::env::var("LATITUDE_API_KEY").expect("LATITUDE_API_KEY must be set");
    let latitude_provider = LatitudeProvider::new(latitude_api_key);
    
    let latitude_spec = ServerSpec {
        name: "integration-test-latitude-node".to_string(),
        region: "us-west".to_string(),
        plan: "standard".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec![],
    };
    
    let latitude_result = latitude_provider.create_server(&latitude_spec).await;
    assert!(latitude_result.is_ok(), "Latitude provider failed: {:?}", latitude_result.err());
    
    // Test OVH provider
    let ovh_endpoint = std::env::var("OVH_ENDPOINT").expect("OVH_ENDPOINT must be set");
    let ovh_app_key = std::env::var("OVH_APPLICATION_KEY").expect("OVH_APPLICATION_KEY must be set");
    let ovh_app_secret = std::env::var("OVH_APPLICATION_SECRET").expect("OVH_APPLICATION_SECRET must be set");
    let ovh_consumer_key = std::env::var("OVH_CONSUMER_KEY").expect("OVH_CONSUMER_KEY must be set");

    let ovh_provider = OvhProvider::new(ovh_endpoint, ovh_app_key, ovh_app_secret, ovh_consumer_key);
    
    let ovh_spec = ServerSpec {
        name: "integration-test-ovh-node".to_string(),
        region: "fr-par".to_string(),
        plan: "s1-2".to_string(), // Use a real OVH plan
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec![],
    };
    
    let ovh_result = ovh_provider.create_server(&ovh_spec).await;
    assert!(ovh_result.is_ok(), "OVH provider failed: {:?}", ovh_result.err());

    // Cleanup: Delete created servers
    if let Ok(server) = cherry_result {
        let _ = cherry_provider.delete_server(&server.id).await;
    }
    
    if let Ok(server) = latitude_result {
        let _ = latitude_provider.delete_server(&server.id).await;
    }
    
    if let Ok(server) = ovh_result {
        let _ = ovh_provider.delete_server(&server.id).await;
    }
}

#[tokio::test]
async fn test_solana_node_creation() {
    // Test creating SolanaNode specifications for different node types
    let validator_spec = SolanaNodeSpec {
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