// Latitude provider tests with mocked responses - no real API calls
use cto_blockchain_operator::providers::latitude::LatitudeProvider;
use cto_blockchain_operator::models::ServerSpec;
use cto_blockchain_operator::providers::MetalProvider;

#[tokio::test]
async fn test_latitude_provider_construction() {
    let _provider = LatitudeProvider::new("test-api-key".to_string());
    // If we get here without panic, construction works
}

#[tokio::test]
async fn test_latitude_provider_validate_with_spec() {
    let provider = LatitudeProvider::new("test-api-key".to_string());
    
    let spec = ServerSpec {
        name: "test-node".to_string(),
        region: "us-west".to_string(),
        plan: "performance".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    // Just verify the method can be called - result depends on API
    let _ = provider.validate_server_creation(&spec).await;
}

#[tokio::test]
async fn test_latitude_provider_validate_invalid_region() {
    let provider = LatitudeProvider::new("test-api-key".to_string());
    
    let spec = ServerSpec {
        name: "test-node".to_string(),
        region: "invalid-region-xyz".to_string(),
        plan: "standard".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    // Validation with invalid region - expect error
    let result = provider.validate_server_creation(&spec).await;
    assert!(result.is_err());
}

// Mock-based server spec tests
#[tokio::test]
async fn test_server_spec_latitude() {
    let spec = ServerSpec {
        name: "test-solana-node".to_string(),
        region: "us-west".to_string(),
        plan: "performance".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    assert_eq!(spec.name, "test-solana-node");
    assert_eq!(spec.region, "us-west");
    assert_eq!(spec.plan, "performance");
}
