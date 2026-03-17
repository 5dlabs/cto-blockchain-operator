//! Tests for Polygon PoS CRD definitions

use cto_blockchain_operator::crds::polygon::*;
use cto_blockchain_operator::crds::PolygonNode;

#[tokio::test]
async fn test_polygon_full_node_crd() {
    let node = PolygonNode {
        metadata: Default::default(),
        spec: PolygonNodeSpec {
            node_type: PolygonNodeType::Full,
            network: PolygonNetwork::Mainnet,
            deployment_target: DeploymentTarget::InCluster,
            bare_metal: None,
            resources: NodeResources {
                cpu_request: "16".to_string(),
                memory_request: "64Gi".to_string(),
                cpu_limit: Some("32".to_string()),
                memory_limit: Some("128Gi".to_string()),
            },
            heimdall: HeimdallConfig {
                image: "0xpolygon/heimdall-v2:0.6.0".to_string(),
                seeds: Some(vec!["seed1@1.2.3.4:26656".to_string()]),
                ..Default::default()
            },
            bor: Some(BorConfig {
                image: "0xpolygon/bor:2.6.3".to_string(),
                bootnodes: Some(vec!["enode://abc@1.2.3.4:30303".to_string()]),
                p2p_port: 30303,
                http_port: 8545,
                ws_port: 8546,
                metrics_port: 9091,
                http_enabled: true,
                http_api: "eth,net,web3,txpool,bor".to_string(),
                ws_enabled: false,
                max_peers: 50,
                extra_args: None,
            }),
            erigon: None,
            storage: StorageConfig {
                storage_class: "csi-cinder-high-speed".to_string(),
                heimdall_size: "1Ti".to_string(),
                execution_size: "8Ti".to_string(),
            },
            enable_metrics: true,
        },
        status: None,
    };

    assert_eq!(node.spec.node_type, PolygonNodeType::Full);
    assert_eq!(node.spec.network, PolygonNetwork::Mainnet);
    assert!(node.spec.bor.is_some());
    assert!(node.spec.erigon.is_none());
}

#[tokio::test]
async fn test_polygon_archive_node_crd() {
    let node = PolygonNode {
        metadata: Default::default(),
        spec: PolygonNodeSpec {
            node_type: PolygonNodeType::Archive,
            network: PolygonNetwork::Mainnet,
            deployment_target: DeploymentTarget::InCluster,
            bare_metal: None,
            resources: NodeResources {
                cpu_request: "16".to_string(),
                memory_request: "128Gi".to_string(),
                cpu_limit: Some("32".to_string()),
                memory_limit: Some("128Gi".to_string()),
            },
            heimdall: HeimdallConfig::default(),
            bor: None,
            erigon: Some(ErigonConfig {
                image: "erigontech/erigon:v3.2.2".to_string(),
                p2p_port: 30303,
                http_port: 8545,
                metrics_port: 9091,
                prune_mode: "archive".to_string(),
                chain: Some("bor-mainnet".to_string()),
                extra_args: None,
            }),
            storage: StorageConfig {
                storage_class: "csi-cinder-high-speed".to_string(),
                heimdall_size: "1Ti".to_string(),
                execution_size: "16Ti".to_string(),
            },
            enable_metrics: true,
        },
        status: None,
    };

    assert_eq!(node.spec.node_type, PolygonNodeType::Archive);
    assert!(node.spec.bor.is_none());
    assert!(node.spec.erigon.is_some());
    assert_eq!(
        node.spec.erigon.as_ref().unwrap().prune_mode,
        "archive"
    );
    assert_eq!(node.spec.storage.execution_size, "16Ti");
}

#[tokio::test]
async fn test_polygon_sentry_node_crd() {
    let node = PolygonNode {
        metadata: Default::default(),
        spec: PolygonNodeSpec {
            node_type: PolygonNodeType::Sentry,
            network: PolygonNetwork::Mainnet,
            deployment_target: DeploymentTarget::InCluster,
            bare_metal: None,
            resources: NodeResources {
                cpu_request: "16".to_string(),
                memory_request: "64Gi".to_string(),
                cpu_limit: None,
                memory_limit: None,
            },
            heimdall: HeimdallConfig::default(),
            bor: Some(BorConfig {
                image: "0xpolygon/bor:2.6.3".to_string(),
                bootnodes: None,
                p2p_port: 30303,
                http_port: 8545,
                ws_port: 8546,
                metrics_port: 9091,
                http_enabled: false,
                http_api: "eth,net,web3,txpool,bor".to_string(),
                ws_enabled: false,
                max_peers: 100,
                extra_args: None,
            }),
            erigon: None,
            storage: StorageConfig {
                storage_class: "csi-cinder-high-speed".to_string(),
                heimdall_size: "1Ti".to_string(),
                execution_size: "8Ti".to_string(),
            },
            enable_metrics: true,
        },
        status: None,
    };

    assert_eq!(node.spec.node_type, PolygonNodeType::Sentry);
    assert!(node.spec.bor.is_some());
    assert!(!node.spec.bor.as_ref().unwrap().http_enabled);
    assert_eq!(node.spec.bor.as_ref().unwrap().max_peers, 100);
}

#[tokio::test]
async fn test_polygon_defaults() {
    let heimdall = HeimdallConfig::default();
    assert_eq!(heimdall.image, "0xpolygon/heimdall-v2:0.6.0");
    assert_eq!(heimdall.p2p_port, 26656);
    assert_eq!(heimdall.rpc_port, 26657);
    assert_eq!(heimdall.rest_port, 1317);
    assert_eq!(heimdall.metrics_port, 26660);
}

#[tokio::test]
async fn test_polygon_serialization_roundtrip() {
    let spec = PolygonNodeSpec {
        node_type: PolygonNodeType::Full,
        network: PolygonNetwork::Amoy,
        deployment_target: DeploymentTarget::InCluster,
        bare_metal: None,
        resources: NodeResources {
            cpu_request: "8".to_string(),
            memory_request: "32Gi".to_string(),
            cpu_limit: None,
            memory_limit: None,
        },
        heimdall: HeimdallConfig::default(),
        bor: Some(BorConfig {
            image: "0xpolygon/bor:2.6.3".to_string(),
            bootnodes: None,
            p2p_port: 30303,
            http_port: 8545,
            ws_port: 8546,
            metrics_port: 9091,
            http_enabled: true,
            http_api: "eth,net,web3".to_string(),
            ws_enabled: false,
            max_peers: 25,
            extra_args: None,
        }),
        erigon: None,
        storage: StorageConfig {
            storage_class: "standard".to_string(),
            heimdall_size: "100Gi".to_string(),
            execution_size: "500Gi".to_string(),
        },
        enable_metrics: false,
    };

    let json = serde_json::to_string(&spec).expect("serialize");
    let deserialized: PolygonNodeSpec = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(spec.node_type, deserialized.node_type);
    assert_eq!(spec.network, deserialized.network);
    assert_eq!(spec.enable_metrics, deserialized.enable_metrics);
    assert_eq!(
        spec.bor.as_ref().unwrap().max_peers,
        deserialized.bor.as_ref().unwrap().max_peers
    );
}

#[tokio::test]
async fn test_polygon_status_defaults() {
    let status = PolygonNodeStatus::default();
    assert!(status.phase.is_none());
    assert!(status.healthy.is_none());
    assert!(status.heimdall_synced.is_none());
    assert!(status.execution_synced.is_none());
    assert!(status.peers.is_none());
    assert!(status.message.is_none());
}

#[tokio::test]
async fn test_polygon_bare_metal_config() {
    let node = PolygonNode {
        metadata: Default::default(),
        spec: PolygonNodeSpec {
            node_type: PolygonNodeType::Full,
            network: PolygonNetwork::Mainnet,
            deployment_target: DeploymentTarget::BareMetal,
            bare_metal: Some(BareMetalConfig {
                provider: "ovh".to_string(),
                server_id: Some("ns1234567.ip-1-2-3.eu".to_string()),
                plan: Some("infra-1".to_string()),
                region: Some("GRA".to_string()),
            }),
            resources: NodeResources {
                cpu_request: "16".to_string(),
                memory_request: "64Gi".to_string(),
                cpu_limit: None,
                memory_limit: None,
            },
            heimdall: HeimdallConfig::default(),
            bor: Some(BorConfig {
                image: "0xpolygon/bor:2.6.3".to_string(),
                bootnodes: None,
                p2p_port: 30303,
                http_port: 8545,
                ws_port: 8546,
                metrics_port: 9091,
                http_enabled: true,
                http_api: "eth,net,web3,txpool,bor".to_string(),
                ws_enabled: false,
                max_peers: 50,
                extra_args: None,
            }),
            erigon: None,
            storage: StorageConfig {
                storage_class: "local-path".to_string(),
                heimdall_size: "1Ti".to_string(),
                execution_size: "8Ti".to_string(),
            },
            enable_metrics: true,
        },
        status: None,
    };

    assert_eq!(node.spec.deployment_target, DeploymentTarget::BareMetal);
    assert!(node.spec.bare_metal.is_some());
    assert_eq!(node.spec.bare_metal.as_ref().unwrap().provider, "ovh");
}
