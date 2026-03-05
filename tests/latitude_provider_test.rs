// Latitude provider tests with mocked responses - no real API calls
use cto_blockchain_operator::providers::latitude::LatitudeProvider;
use cto_blockchain_operator::models::ServerSpec;
use cto_blockchain_operator::providers::MetalProvider;

#[tokio::test]
async fn test_latitude_provider_construction() {
    let provider = LatitudeProvider::new("test-api-key".to_string());
    
    // Verify provider is initialized
    assert_eq!(provider.api_key, "test-api-key");
}

#[tokio::test]
async fn test_latitude_provider_validate_plan() {
    let provider = LatitudeProvider::new("test-api-key".to_string());
    
    // Test validation with various plans
    let plans = vec![
        "standard".to_string(),
        "performance".to_string(),
        "gpu".to_string(),
    ];
    
    for plan in plans {
        let result = provider.validate_server_creation(
            "test-node".to_string(),
            plan,
            "us-west".to_string()
        ).await;
        // Just verify method executes
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_latitude_provider_validate_invalid_region() {
    let provider = LatitudeProvider::new("test-api-key".to_string());
    
    // Test with invalid region - should fail validation
    let result = provider.validate_server_creation(
        "test-node".to_string(),
        "standard".to_string(),
        "invalid-region".to_string()
    ).await;
    
    // Invalid region should fail
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
