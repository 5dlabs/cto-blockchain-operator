use cto_blockchain_operator::controllers::solana::{
    build_service, build_statefulset, validate_external_cluster_config, ControllerError,
};
use cto_blockchain_operator::crds::{
    DeploymentMode, ExternalClusterMode, ExternalClusterSpec, NodeConfig, NodeResources,
    NodeType, Provider, SolanaNode, SolanaNodeSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

// ── helpers ──────────────────────────────────────────────────────────────────

fn default_resources() -> NodeResources {
    NodeResources {
        cpu_request: "28".to_string(),
        memory_request: "64Gi".to_string(),
        cpu_limit: None,
        memory_limit: None,
    }
}

fn default_config() -> NodeConfig {
    NodeConfig {
        expected_genesis_hash: "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d".to_string(),
        limit_ledger_size: 200_000_000,
        full_rpc_api: true,
        enable_accounts_disk_index: true,
        skip_startup_ledger_verification: true,
        rpc_threads: 128,
        maximum_full_snapshots_to_retain: 2,
        wal_recovery_mode: "skip_any_corrupted_record".to_string(),
    }
}

fn make_solana_node(name: &str, deployment_mode: DeploymentMode) -> SolanaNode {
    SolanaNode {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some("default".to_string()),
            ..Default::default()
        },
        spec: SolanaNodeSpec {
            deployment_mode,
            node_pools: vec![],
            external_cluster: None,
            provider: Provider::Cherry,
            region: "nl-ams".to_string(),
            replicas: 1,
            node_type: NodeType::Validator,
            rpc_port: 8899,
            gossip_port: 8001,
            resources: default_resources(),
            config: default_config(),
            image: "anzaxyz/agave:v3.1.9".to_string(),
            enable_voting: false,
            identity_secret: "test-identity".to_string(),
            known_validators: None,
            entrypoints: None,
        },
        status: None,
    }
}

fn make_ext_spec(
    mode: ExternalClusterMode,
    cluster_name: Option<&str>,
    endpoint: Option<&str>,
    ssh_keys: Vec<String>,
) -> ExternalClusterSpec {
    ExternalClusterSpec {
        provider: Provider::Cherry,
        mode,
        region_preferences: vec!["nl-ams".to_string()],
        existing_cluster_name: cluster_name.map(str::to_string),
        existing_cluster_endpoint: endpoint.map(str::to_string),
        create_k8s_cluster: false,
        ssh_keys,
    }
}

// ── SolanaNodeSpec construction ───────────────────────────────────────────────

#[test]
fn test_solana_node_spec_defaults() {
    let node = make_solana_node("test", DeploymentMode::InCluster);
    assert_eq!(node.spec.provider, Provider::Cherry);
    assert_eq!(node.spec.replicas, 1);
    assert_eq!(node.spec.node_type, NodeType::Validator);
    assert_eq!(node.spec.rpc_port, 8899);
    assert_eq!(node.spec.gossip_port, 8001);
    assert!(!node.spec.enable_voting);
}

// ── build_statefulset ─────────────────────────────────────────────────────────

#[test]
fn build_statefulset_has_correct_name_and_namespace() {
    let node = make_solana_node("my-validator", DeploymentMode::InCluster);
    let sts = build_statefulset(&node);
    assert_eq!(sts.metadata.name.as_deref(), Some("my-validator"));
    assert_eq!(sts.metadata.namespace.as_deref(), Some("default"));
}

#[test]
fn build_statefulset_replica_count_matches_spec() {
    let mut node = make_solana_node("val", DeploymentMode::InCluster);
    node.spec.replicas = 3;
    let sts = build_statefulset(&node);
    assert_eq!(sts.spec.as_ref().unwrap().replicas, Some(3));
}

#[test]
fn build_statefulset_contains_rpc_and_gossip_ports() {
    let mut node = make_solana_node("val", DeploymentMode::InCluster);
    node.spec.rpc_port = 9000;
    node.spec.gossip_port = 9001;
    let sts = build_statefulset(&node);
    let containers = &sts.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers;
    let ports = containers[0].ports.as_ref().unwrap();
    let port_names: Vec<_> = ports.iter().filter_map(|p| p.name.as_deref()).collect();
    assert!(port_names.contains(&"rpc"), "expected rpc port");
    assert!(port_names.contains(&"gossip"), "expected gossip port");
    let rpc = ports.iter().find(|p| p.name.as_deref() == Some("rpc")).unwrap();
    assert_eq!(rpc.container_port, 9000);
    let gossip = ports.iter().find(|p| p.name.as_deref() == Some("gossip")).unwrap();
    assert_eq!(gossip.container_port, 9001);
}

#[test]
fn build_statefulset_includes_identity_secret_volume() {
    let node = make_solana_node("val", DeploymentMode::InCluster);
    let sts = build_statefulset(&node);
    let volumes = sts
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .volumes
        .as_ref()
        .unwrap();
    let identity_vol = volumes.iter().find(|v| v.name == "identity");
    assert!(identity_vol.is_some(), "identity volume must exist");
    let secret_name = identity_vol
        .unwrap()
        .secret
        .as_ref()
        .unwrap()
        .secret_name
        .as_deref();
    assert_eq!(secret_name, Some("test-identity"));
}

#[test]
fn build_statefulset_args_contain_genesis_hash() {
    let node = make_solana_node("val", DeploymentMode::InCluster);
    let sts = build_statefulset(&node);
    let args = sts
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0]
        .args
        .as_ref()
        .unwrap();
    let genesis_pos = args.iter().position(|a| a == "--expected-genesis-hash");
    assert!(genesis_pos.is_some(), "--expected-genesis-hash flag missing");
    assert_eq!(
        args[genesis_pos.unwrap() + 1],
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
    );
}

#[test]
fn build_statefulset_no_voting_flag_when_disabled() {
    let node = make_solana_node("val", DeploymentMode::InCluster);
    assert!(!node.spec.enable_voting);
    let sts = build_statefulset(&node);
    let args = sts
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0]
        .args
        .as_ref()
        .unwrap();
    assert!(args.contains(&"--no-voting".to_string()), "--no-voting must be present when voting disabled");
}

#[test]
fn build_statefulset_no_voting_flag_absent_when_enabled() {
    let mut node = make_solana_node("val", DeploymentMode::InCluster);
    node.spec.enable_voting = true;
    let sts = build_statefulset(&node);
    let args = sts
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0]
        .args
        .as_ref()
        .unwrap();
    assert!(!args.contains(&"--no-voting".to_string()), "--no-voting must be absent when voting enabled");
}

#[test]
fn build_statefulset_has_ledger_pvc() {
    let node = make_solana_node("val", DeploymentMode::InCluster);
    let sts = build_statefulset(&node);
    let pvcs = sts.spec.as_ref().unwrap().volume_claim_templates.as_ref().unwrap();
    let ledger_pvc = pvcs.iter().find(|p| p.metadata.name.as_deref() == Some("ledger"));
    assert!(ledger_pvc.is_some(), "ledger PVC must exist");
}

#[test]
fn build_statefulset_resource_labels_contain_instance() {
    let node = make_solana_node("my-node", DeploymentMode::InCluster);
    let sts = build_statefulset(&node);
    let labels = sts.metadata.labels.as_ref().unwrap();
    assert_eq!(
        labels.get("app.kubernetes.io/instance").map(String::as_str),
        Some("my-node")
    );
    assert_eq!(
        labels.get("app.kubernetes.io/name").map(String::as_str),
        Some("solana-node")
    );
}

// ── build_service ─────────────────────────────────────────────────────────────

#[test]
fn build_service_has_correct_name_and_namespace() {
    let node = make_solana_node("my-validator", DeploymentMode::InCluster);
    let svc = build_service(&node);
    assert_eq!(svc.metadata.name.as_deref(), Some("my-validator"));
    assert_eq!(svc.metadata.namespace.as_deref(), Some("default"));
}

#[test]
fn build_service_is_headless() {
    let node = make_solana_node("val", DeploymentMode::InCluster);
    let svc = build_service(&node);
    assert_eq!(
        svc.spec.as_ref().unwrap().cluster_ip.as_deref(),
        Some("None"),
        "service must be headless (clusterIP=None)"
    );
}

#[test]
fn build_service_exposes_rpc_and_gossip() {
    let mut node = make_solana_node("val", DeploymentMode::InCluster);
    node.spec.rpc_port = 9000;
    node.spec.gossip_port = 9001;
    let svc = build_service(&node);
    let ports = svc.spec.as_ref().unwrap().ports.as_ref().unwrap();
    let rpc = ports.iter().find(|p| p.name.as_deref() == Some("rpc")).unwrap();
    assert_eq!(rpc.port, 9000);
    let gossip = ports.iter().find(|p| p.name.as_deref() == Some("gossip")).unwrap();
    assert_eq!(gossip.port, 9001);
}

// ── validate_external_cluster_config ─────────────────────────────────────────

#[test]
fn validate_add_worker_ok_when_cluster_name_set() {
    let ext = Some(make_ext_spec(
        ExternalClusterMode::AddWorkerToExistingCluster,
        Some("prod-cluster"),
        None,
        vec![],
    ));
    let result =
        validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
    assert!(result.is_ok());
}

#[test]
fn validate_add_worker_ok_when_endpoint_set() {
    let ext = Some(make_ext_spec(
        ExternalClusterMode::AddWorkerToExistingCluster,
        None,
        Some("https://10.0.0.1:6443"),
        vec![],
    ));
    let result =
        validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
    assert!(result.is_ok());
}

#[test]
fn validate_add_worker_errors_without_cluster_info() {
    let ext = Some(make_ext_spec(
        ExternalClusterMode::AddWorkerToExistingCluster,
        None,
        None,
        vec![],
    ));
    let result =
        validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
    assert!(
        matches!(result, Err(ControllerError::InvalidConfig(_))),
        "expected InvalidConfig, got {:?}",
        result
    );
    if let Err(ControllerError::InvalidConfig(msg)) = result {
        assert!(
            msg.contains("existing_cluster_name"),
            "error message should mention existing_cluster_name"
        );
    }
}

#[test]
fn validate_provision_new_cluster_ok_without_cluster_info() {
    // ProvisionNewCluster does not need existing cluster references
    let ext = Some(make_ext_spec(
        ExternalClusterMode::ProvisionNewCluster,
        None,
        None,
        vec![],
    ));
    let result =
        validate_external_cluster_config(&ext, &ExternalClusterMode::ProvisionNewCluster);
    assert!(result.is_ok());
}

#[test]
fn validate_none_ext_always_ok_regardless_of_mode() {
    // Legacy path: no ExternalClusterSpec → skip validation
    let result_add =
        validate_external_cluster_config(&None, &ExternalClusterMode::AddWorkerToExistingCluster);
    let result_new =
        validate_external_cluster_config(&None, &ExternalClusterMode::ProvisionNewCluster);
    assert!(result_add.is_ok());
    assert!(result_new.is_ok());
}

// ── K8s-dependent tests (require a live cluster) ──────────────────────────────

/// This test requires a running Kubernetes cluster and valid kubeconfig.
/// Run with: cargo test -- --ignored
#[tokio::test]
#[ignore]
async fn test_solana_controller_reconcile_integration() {
    use cto_blockchain_operator::controllers::solana::SolanaController;
    use kube::Client;

    let client = Client::try_default().await.unwrap();
    let controller = SolanaController::new(client);
    let node = make_solana_node("integration-test", DeploymentMode::InCluster);
    let result = controller.reconcile(&node).await;
    // Result may fail without actual resources, but reconcile should not panic
    let _ = result;
}
