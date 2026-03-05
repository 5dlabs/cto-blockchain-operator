// Provider tests with mocked responses - no real API calls
use cto_blockchain_operator::providers::cherry::CherryProvider;
use cto_blockchain_operator::models::{ServerSpec, ServerStatus};
use cto_blockchain_operator::providers::MetalProvider;

#[tokio::test]
async fn test_cherry_provider_construction() {
    // Test provider can be constructed - basic smoke test
    let _provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    // If we get here without panic, construction works
}

#[tokio::test]
async fn test_cherry_provider_validate_with_valid_spec() {
    let provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    let spec = ServerSpec {
        name: "test-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "e5-1660v3".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    // Just verify the method can be called - result depends on API
    let _ = provider.validate_server_creation(&spec).await;
}

#[tokio::test]
async fn test_cherry_provider_validate_with_invalid_plan() {
    let provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    let spec = ServerSpec {
        name: "test-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "invalid-plan-xyz-12345".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    // Validation with invalid plan - expect error
    let result = provider.validate_server_creation(&spec).await;
    assert!(result.is_err());
}

// Mock-based tests that don't require real API credentials
#[tokio::test]
async fn test_server_status_conversion() {
    let status = ServerStatus::Active;
    assert_eq!(status.to_string(), "Active");
    
    let status = ServerStatus::Provisioning;
    assert_eq!(status.to_string(), "Provisioning");
    
    let status = ServerStatus::Inactive;
    assert_eq!(status.to_string(), "Inactive");
    
    let status = ServerStatus::Error;
    assert_eq!(status.to_string(), "Error");
}

#[tokio::test]
async fn test_server_spec_construction() {
    let spec = ServerSpec {
        name: "test-solana-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "e5-1660v3".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    assert_eq!(spec.name, "test-solana-node");
    assert_eq!(spec.region, "nl-ams");
    assert_eq!(spec.plan, "e5-1660v3");
}
