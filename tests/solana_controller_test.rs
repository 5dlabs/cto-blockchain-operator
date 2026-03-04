use cto_blockchain_operator::controllers::solana::{build_service, build_statefulset};
use cto_blockchain_operator::crds::{
    NodeConfig, NodeResources, NodeType, SolanaNode, SolanaNodeSpec,
};
use kube::api::ObjectMeta;

fn test_solana_node() -> SolanaNode {
    SolanaNode {
        metadata: ObjectMeta {
            name: Some("test-validator".to_string()),
            namespace: Some("cto".to_string()),
            ..Default::default()
        },
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
            identity_secret: "solana-identity".to_string(),
            known_validators: Some(vec![
                "validator111111111111111111111111111111111".to_string()
            ]),
            entrypoints: Some(vec!["entrypoint.mainnet-beta.solana.com:8001".to_string()]),
        },
        status: None,
    }
}

#[test]
fn test_build_statefulset() {
    let node = test_solana_node();
    let st = build_statefulset(&node);

    let meta = st.metadata;
    assert_eq!(meta.name.as_deref(), Some("test-validator"));
    assert_eq!(meta.namespace.as_deref(), Some("cto"));

    let spec = st.spec.expect("statefulset spec missing");
    assert_eq!(spec.replicas, Some(1));
    assert_eq!(spec.service_name, "test-validator");

    let container = &spec.template.spec.expect("pod spec missing").containers[0];

    assert_eq!(container.name, "solana");
    assert_eq!(container.image.as_deref(), Some("anzaxyz/agave:v3.1.9"));

    let args = container.args.as_ref().expect("args missing");
    assert!(args.contains(&"--identity".to_string()));
    assert!(args.contains(&"--no-voting".to_string()));
    assert!(args.contains(&"--known-validator".to_string()));
    assert!(args.contains(&"--entrypoint".to_string()));

    let claims = spec
        .volume_claim_templates
        .expect("volume claim templates missing");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].metadata.name.as_deref(), Some("ledger"));
}

#[test]
fn test_build_service() {
    let node = test_solana_node();
    let svc = build_service(&node);

    let meta = svc.metadata;
    assert_eq!(meta.name.as_deref(), Some("test-validator"));
    assert_eq!(meta.namespace.as_deref(), Some("cto"));

    let spec = svc.spec.expect("service spec missing");
    assert_eq!(spec.cluster_ip.as_deref(), Some("None"));

    let ports = spec.ports.expect("service ports missing");
    assert_eq!(ports.len(), 2);
    assert!(ports
        .iter()
        .any(|p| p.name.as_deref() == Some("rpc") && p.port == 8899));
    assert!(ports
        .iter()
        .any(|p| p.name.as_deref() == Some("gossip") && p.port == 8001));
}
