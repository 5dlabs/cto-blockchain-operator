use cto_blockchain_operator::providers::cherry::CherryProvider;
use cto_blockchain_operator::providers::{MetalProvider, ServerSpec};

#[tokio::test]
async fn test_cherry_provider_create_server() {
    let provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    let spec = ServerSpec {
        name: "test-solana-node".to_string(),
        region: "nl-ams".to_string(),
        plan: "solana-server-gen5".to_string(),
        image: "ubuntu_22_04".to_string(),
        ssh_keys: vec!["test-key".to_string()],
    };
    
    let result = provider.create_server(&spec).await;
    assert!(result.is_ok());
    
    let server = result.unwrap();
    assert_eq!(server.hostname, "test-solana-node");
    assert_eq!(server.region, "nl-ams");
}

#[tokio::test]
async fn test_cherry_provider_get_server() {
    let provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    let result = provider.get_server("test-server-id").await;
    assert!(result.is_ok());
    
    let server = result.unwrap();
    assert_eq!(server.id, "test-server-id");
    assert_eq!(server.status.to_string(), "Active");
}

#[tokio::test]
async fn test_cherry_provider_operations() {
    let provider = CherryProvider::new(
        "test-api-key".to_string(),
        "test-team-id".to_string(),
        "test-project-id".to_string(),
    );
    
    // Test start server
    let start_result = provider.start_server("test-server-id").await;
    assert!(start_result.is_ok());
    
    // Test stop server
    let stop_result = provider.stop_server("test-server-id").await;
    assert!(stop_result.is_ok());
    
    // Test delete server
    let delete_result = provider.delete_server("test-server-id").await;
    assert!(delete_result.is_ok());
    
    // Test list servers
    let list_result = provider.list_servers().await;
    assert!(list_result.is_ok());
}