use std::collections::BTreeMap;
use std::sync::Arc;

use crate::crds::{
    DeploymentMode, ExternalClusterMode, NodePhase, NodePoolRole, Provider as CrdProvider,
    SolanaNode, SolanaNodeStatus,
};
use crate::models::ServerSpec;
use crate::providers::{CherryProvider, LatitudeProvider, MetalProvider, OvhProvider};
use k8s_openapi::api::apps::v1::{
    StatefulSet, StatefulSetSpec, StatefulSetStatus, StatefulSetUpdateStrategy,
};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, PersistentVolumeClaim, PersistentVolumeClaimSpec, PodSpec,
    PodTemplateSpec, ResourceRequirements, SecretVolumeSource, Service, ServicePort, ServiceSpec,
    Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Resource, ResourceExt};
use thiserror::Error;
use tracing::*;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("Failed to provision server: {0}")]
    ProvisionError(String),
    #[error("Failed to create Kubernetes resources: {0}")]
    K8sError(String),
    #[error("Node in error state: {0}")]
    NodeError(String),
    #[error("Kubernetes API error: {0}")]
    Kubernetes(#[from] kube::Error),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub struct SolanaController {
    client: Client,
}

impl SolanaController {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn reconcile(&self, crd: &SolanaNode) -> Result<SolanaNodeStatus, ControllerError> {
        let name = crd.name_any();
        let namespace = crd.namespace().unwrap_or_else(|| "default".to_string());

        let mode_label = match crd.spec.deployment_mode {
            DeploymentMode::InCluster => "in-cluster",
            DeploymentMode::External => "external",
        };
        info!(name = %name, ns = %namespace, mode = mode_label, "Reconcile begin");

        match crd.spec.deployment_mode {
            DeploymentMode::InCluster => {
                create_statefulset(&self.client, crd).await?;
                create_service(&self.client, crd).await?;

                let statefulsets: Api<StatefulSet> = Api::namespaced(self.client.clone(), &namespace);
                let services: Api<Service> = Api::namespaced(self.client.clone(), &namespace);

                let statefulset = statefulsets.get_opt(&name).await?;
                let service = services.get_opt(&name).await?;

                let status = observe_status(
                    statefulset.as_ref(),
                    service.as_ref(),
                    crd.spec.replicas,
                );
                info!(
                    name = %name,
                    ns = %namespace,
                    phase = ?status.phase,
                    healthy = status.healthy.unwrap_or(false),
                    "Reconcile complete (in-cluster)"
                );
                Ok(status)
            }
            DeploymentMode::External => {
                let ext = crd.spec.external_cluster.clone();

                let mode = ext
                    .as_ref()
                    .map(|e| e.mode.clone())
                    .unwrap_or(ExternalClusterMode::AddWorkerToExistingCluster);

                let ext_mode_label = match mode {
                    ExternalClusterMode::AddWorkerToExistingCluster => "add-worker",
                    ExternalClusterMode::ProvisionNewCluster => "provision-new-cluster",
                };
                info!(name = %name, ext_mode = ext_mode_label, "External mode selected");

                // Validate configuration before any provisioning side-effects.
                validate_external_cluster_config(&ext, &mode)?;

                let provider_kind = ext
                    .as_ref()
                    .map(|e| e.provider.clone())
                    .unwrap_or_else(|| crd.spec.provider.clone());

                let provider = build_provider(&provider_kind)?;

                let region = ext
                    .as_ref()
                    .and_then(|e| e.region_preferences.first().cloned())
                    .unwrap_or_else(|| crd.spec.region.clone());

                let ssh_keys = ext
                    .as_ref()
                    .map(|e| e.ssh_keys.clone())
                    .unwrap_or_default();

                if ssh_keys.is_empty() {
                    warn!(
                        name = %name,
                        "No SSH keys configured for external provisioning — \
                         servers will be unreachable without key-based SSH access"
                    );
                }

                let created = provision_node_pools(provider.as_ref(), crd, &region, &ssh_keys).await?;

                match mode {
                    ExternalClusterMode::AddWorkerToExistingCluster => {
                        info!(
                            name = %name,
                            created,
                            "Provisioned node(s) for existing external cluster join"
                        );
                        Ok(SolanaNodeStatus {
                            phase: Some(NodePhase::Initializing),
                            slot_height: None,
                            healthy: Some(false),
                            slots_behind: None,
                        })
                    }
                    ExternalClusterMode::ProvisionNewCluster => {
                        let auto_bootstrap = ext.as_ref().map(|e| e.create_k8s_cluster).unwrap_or(false);
                        if !auto_bootstrap {
                            warn!(
                                name = %name,
                                "ProvisionNewCluster with create_k8s_cluster=false: \
                                 nodes provisioned but Kubernetes bootstrapping must be \
                                 performed manually"
                            );
                        }
                        info!(
                            name = %name,
                            created,
                            "Provisioned node(s) for new external cluster bootstrap"
                        );
                        Ok(SolanaNodeStatus {
                            phase: Some(NodePhase::Pending),
                            slot_height: None,
                            healthy: Some(false),
                            slots_behind: None,
                        })
                    }
                }
            }
        }
    }
}

/// Validate the external cluster configuration for the requested mode.
///
/// This must be called before any provisioning side-effects so that obvious
/// misconfigurations are rejected immediately with a clear error.
pub fn validate_external_cluster_config(
    ext: &Option<crate::crds::ExternalClusterSpec>,
    mode: &ExternalClusterMode,
) -> Result<(), ControllerError> {
    let Some(spec) = ext.as_ref() else {
        // Legacy path: external_cluster not set, no spec-level validation possible.
        return Ok(());
    };

    match mode {
        ExternalClusterMode::AddWorkerToExistingCluster => {
            if spec.existing_cluster_name.is_none() && spec.existing_cluster_endpoint.is_none() {
                return Err(ControllerError::InvalidConfig(
                    "mode=add-worker-to-existing-cluster requires at least one of: \
                     existing_cluster_name, existing_cluster_endpoint"
                        .into(),
                ));
            }
            Ok(())
        }
        ExternalClusterMode::ProvisionNewCluster => Ok(()),
    }
}

fn build_provider(kind: &CrdProvider) -> Result<Arc<dyn MetalProvider>, ControllerError> {
    let provider_label = match kind {
        CrdProvider::Cherry => "cherry",
        CrdProvider::Latitude => "latitude",
        CrdProvider::Ovh => "ovh",
    };
    info!(provider = provider_label, "Building metal provider");
    match kind {
        CrdProvider::Cherry => {
            let api_key = std::env::var("CHERRY_API_KEY").map_err(|_| {
                ControllerError::ProvisionError("Missing CHERRY_API_KEY for Cherry provider".into())
            })?;
            let team_id = std::env::var("CHERRY_TEAM_ID").unwrap_or_else(|_| "190658".to_string());
            let project_id =
                std::env::var("CHERRY_PROJECT_ID").unwrap_or_else(|_| "264136".to_string());
            Ok(Arc::new(CherryProvider::new(api_key, team_id, project_id)))
        }
        CrdProvider::Latitude => {
            let api_key = std::env::var("LATITUDE_API_KEY").map_err(|_| {
                ControllerError::ProvisionError("Missing LATITUDE_API_KEY for Latitude provider".into())
            })?;
            Ok(Arc::new(LatitudeProvider::new(api_key)))
        }
        CrdProvider::Ovh => {
            let endpoint =
                std::env::var("OVH_ENDPOINT").unwrap_or_else(|_| "ovh-us".to_string());
            let app_key = std::env::var("OVH_APPLICATION_KEY")
                .or_else(|_| std::env::var("OVH_APP_KEY"))
                .map_err(|_| {
                    ControllerError::ProvisionError(
                        "Missing OVH_APPLICATION_KEY/OVH_APP_KEY for OVH provider".into(),
                    )
                })?;
            let app_secret = std::env::var("OVH_APPLICATION_SECRET")
                .or_else(|_| std::env::var("OVH_APP_SECRET"))
                .map_err(|_| {
                    ControllerError::ProvisionError(
                        "Missing OVH_APPLICATION_SECRET/OVH_APP_SECRET for OVH provider".into(),
                    )
                })?;
            let consumer_key = std::env::var("OVH_CONSUMER_KEY").map_err(|_| {
                ControllerError::ProvisionError("Missing OVH_CONSUMER_KEY for OVH provider".into())
            })?;

            Ok(Arc::new(OvhProvider::new(
                endpoint,
                app_key,
                app_secret,
                consumer_key,
            )))
        }
    }
}

async fn provision_node_pools(
    provider: &dyn MetalProvider,
    solana_node: &SolanaNode,
    region: &str,
    ssh_keys: &[String],
) -> Result<usize, ControllerError> {
    let name = solana_node.name_any();

    let pools = if solana_node.spec.node_pools.is_empty() {
        vec![
            (
                NodePoolRole::SolanaRpc,
                1,
                "e5-1660v3".to_string(),
                "ubuntu_22_04".to_string(),
            ),
            (
                NodePoolRole::SupportServices,
                1,
                "e5-1660v3".to_string(),
                "ubuntu_22_04".to_string(),
            ),
        ]
    } else {
        let mut result: Vec<(NodePoolRole, i32, String, String)> = Vec::with_capacity(solana_node.spec.node_pools.len());
        for p in &solana_node.spec.node_pools {
            if p.replicas <= 0 {
                return Err(ControllerError::InvalidConfig(format!(
                    "node pool {:?} has invalid replicas: {} (must be >= 1)",
                    p.role, p.replicas
                )));
            }
            let plan = match p.role {
                NodePoolRole::SolanaRpc => "e5-1660v3".to_string(),
                NodePoolRole::SupportServices => "e5-1660v3".to_string(),
            };
            let image = p
                .config
                .as_ref()
                .and_then(|c| c.image.clone())
                .unwrap_or_else(|| "ubuntu_22_04".to_string());
            result.push((p.role.clone(), p.replicas, plan, image));
        }
        result
    };

    let mut created = 0usize;

    for (role, replicas, plan, image) in pools {
        let role_name = match role {
            NodePoolRole::SolanaRpc => "solana-rpc",
            NodePoolRole::SupportServices => "support-services",
        };

        for i in 0..replicas {
            let server_name = format!("{}-{}-{}", name, role_name, i + 1);
            let spec = ServerSpec {
                name: server_name,
                region: region.to_string(),
                plan: plan.clone(),
                image: image.clone(),
                ssh_keys: ssh_keys.to_vec(),
            };

            info!(
                server = %spec.name,
                role = role_name,
                plan = %spec.plan,
                region = %spec.region,
                "Provisioning server"
            );
            provider
                .create_server(&spec)
                .await
                .map_err(|e| ControllerError::ProvisionError(e.to_string()))?;
            info!(server = %spec.name, "Server provisioned");
            created += 1;
        }
    }

    Ok(created)
}

async fn create_statefulset(client: &Client, solana_node: &SolanaNode) -> Result<(), ControllerError> {
    let namespace = solana_node
        .namespace()
        .unwrap_or_else(|| "default".to_string());
    let name = solana_node.name_any();
    let statefulset = build_statefulset(solana_node);
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);

    debug!(name = %name, ns = %namespace, "Applying StatefulSet");
    api.patch(
        &name,
        &PatchParams::apply("cto-blockchain-operator").force(),
        &Patch::Apply(statefulset),
    )
    .await?;
    info!(name = %name, ns = %namespace, "StatefulSet applied");

    Ok(())
}

async fn create_service(client: &Client, solana_node: &SolanaNode) -> Result<(), ControllerError> {
    let namespace = solana_node
        .namespace()
        .unwrap_or_else(|| "default".to_string());
    let name = solana_node.name_any();
    let service = build_service(solana_node);
    let api: Api<Service> = Api::namespaced(client.clone(), &namespace);

    debug!(name = %name, ns = %namespace, "Applying Service");
    api.patch(
        &name,
        &PatchParams::apply("cto-blockchain-operator").force(),
        &Patch::Apply(service),
    )
    .await?;
    info!(name = %name, ns = %namespace, "Service applied");

    Ok(())
}

pub fn build_statefulset(solana_node: &SolanaNode) -> StatefulSet {
    let namespace = solana_node
        .namespace()
        .unwrap_or_else(|| "default".to_string());
    let name = solana_node.name_any();
    let labels = resource_labels(&name);

    let mut args = vec![
        "--identity".to_string(),
        "/keys/identity.json".to_string(),
        "--ledger".to_string(),
        "/ledger".to_string(),
        "--rpc-port".to_string(),
        solana_node.spec.rpc_port.to_string(),
        "--gossip-port".to_string(),
        solana_node.spec.gossip_port.to_string(),
        "--expected-genesis-hash".to_string(),
        solana_node.spec.config.expected_genesis_hash.clone(),
        "--limit-ledger-size".to_string(),
        solana_node.spec.config.limit_ledger_size.to_string(),
        "--rpc-threads".to_string(),
        solana_node.spec.config.rpc_threads.to_string(),
        "--maximum-full-snapshots-to-retain".to_string(),
        solana_node
            .spec
            .config
            .maximum_full_snapshots_to_retain
            .to_string(),
        "--wal-recovery-mode".to_string(),
        solana_node.spec.config.wal_recovery_mode.clone(),
    ];

    if solana_node.spec.config.full_rpc_api {
        args.push("--full-rpc-api".to_string());
    }
    if solana_node.spec.config.enable_accounts_disk_index {
        args.push("--enable-accounts-disk-index".to_string());
    }
    if solana_node.spec.config.skip_startup_ledger_verification {
        args.push("--skip-startup-ledger-verification".to_string());
    }
    if !solana_node.spec.enable_voting {
        args.push("--no-voting".to_string());
    }
    if let Some(known_validators) = &solana_node.spec.known_validators {
        for validator in known_validators {
            args.push("--known-validator".to_string());
            args.push(validator.clone());
        }
    }
    if let Some(entrypoints) = &solana_node.spec.entrypoints {
        for entrypoint in entrypoints {
            args.push("--entrypoint".to_string());
            args.push(entrypoint.clone());
        }
    }

    let mut requests = BTreeMap::new();
    requests.insert(
        "cpu".to_string(),
        Quantity(solana_node.spec.resources.cpu_request.clone()),
    );
    requests.insert(
        "memory".to_string(),
        Quantity(solana_node.spec.resources.memory_request.clone()),
    );

    let mut limits = BTreeMap::new();
    if let Some(cpu_limit) = &solana_node.spec.resources.cpu_limit {
        limits.insert("cpu".to_string(), Quantity(cpu_limit.clone()));
    }
    if let Some(memory_limit) = &solana_node.spec.resources.memory_limit {
        limits.insert("memory".to_string(), Quantity(memory_limit.clone()));
    }

    let mut metadata = ObjectMeta {
        name: Some(name.clone()),
        namespace: Some(namespace),
        labels: Some(labels.clone()),
        ..Default::default()
    };
    if let Some(owner) = solana_node.controller_owner_ref(&()) {
        metadata.owner_references = Some(vec![owner]);
    }

    StatefulSet {
        metadata,
        spec: Some(StatefulSetSpec {
            replicas: Some(solana_node.spec.replicas),
            service_name: name.clone(),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "solana".to_string(),
                        image: Some(solana_node.spec.image.clone()),
                        args: Some(args),
                        ports: Some(vec![
                            ContainerPort {
                                name: Some("rpc".to_string()),
                                container_port: solana_node.spec.rpc_port,
                                ..Default::default()
                            },
                            ContainerPort {
                                name: Some("gossip".to_string()),
                                container_port: solana_node.spec.gossip_port,
                                ..Default::default()
                            },
                        ]),
                        resources: Some(ResourceRequirements {
                            requests: Some(requests),
                            limits: if limits.is_empty() { None } else { Some(limits) },
                            ..Default::default()
                        }),
                        volume_mounts: Some(vec![
                            VolumeMount {
                                name: "identity".to_string(),
                                mount_path: "/keys".to_string(),
                                read_only: Some(true),
                                ..Default::default()
                            },
                            VolumeMount {
                                name: "ledger".to_string(),
                                mount_path: "/ledger".to_string(),
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    }],
                    volumes: Some(vec![Volume {
                        name: "identity".to_string(),
                        secret: Some(SecretVolumeSource {
                            secret_name: Some(solana_node.spec.identity_secret.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
            },
            volume_claim_templates: Some(vec![PersistentVolumeClaim {
                metadata: ObjectMeta {
                    name: Some("ledger".to_string()),
                    ..Default::default()
                },
                spec: Some(PersistentVolumeClaimSpec {
                    access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                    resources: Some(ResourceRequirements {
                        requests: Some(BTreeMap::from([(
                            "storage".to_string(),
                            Quantity("500Gi".to_string()),
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }]),
            update_strategy: Some(StatefulSetUpdateStrategy::default()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn build_service(solana_node: &SolanaNode) -> Service {
    let namespace = solana_node
        .namespace()
        .unwrap_or_else(|| "default".to_string());
    let name = solana_node.name_any();
    let labels = resource_labels(&name);

    let mut metadata = ObjectMeta {
        name: Some(name),
        namespace: Some(namespace),
        labels: Some(labels.clone()),
        ..Default::default()
    };
    if let Some(owner) = solana_node.controller_owner_ref(&()) {
        metadata.owner_references = Some(vec![owner]);
    }

    Service {
        metadata,
        spec: Some(ServiceSpec {
            selector: Some(labels),
            cluster_ip: Some("None".to_string()),
            ports: Some(vec![
                ServicePort {
                    name: Some("rpc".to_string()),
                    port: solana_node.spec.rpc_port,
                    target_port: Some(IntOrString::String("rpc".to_string())),
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ServicePort {
                    name: Some("gossip".to_string()),
                    port: solana_node.spec.gossip_port,
                    target_port: Some(IntOrString::String("gossip".to_string())),
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn resource_labels(name: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "app.kubernetes.io/name".to_string(),
            "solana-node".to_string(),
        ),
        ("app.kubernetes.io/instance".to_string(), name.to_string()),
    ])
}

fn observe_status(
    statefulset: Option<&StatefulSet>,
    service: Option<&Service>,
    desired_replicas: i32,
) -> SolanaNodeStatus {
    if statefulset.is_none() || service.is_none() {
        return SolanaNodeStatus {
            phase: Some(NodePhase::Pending),
            slot_height: None,
            healthy: Some(false),
            slots_behind: None,
        };
    }

    let status = statefulset
        .and_then(|s| s.status.as_ref())
        .cloned()
        .unwrap_or_default();

    phase_from_statefulset_status(&status, desired_replicas)
}

fn phase_from_statefulset_status(
    status: &StatefulSetStatus,
    desired_replicas: i32,
) -> SolanaNodeStatus {
    let ready = status.ready_replicas.unwrap_or(0);
    let current = status.current_replicas.unwrap_or(0);
    let healthy = ready >= desired_replicas && desired_replicas > 0;

    let phase = if healthy {
        NodePhase::Running
    } else if current > 0 || status.observed_generation.is_some() {
        NodePhase::Initializing
    } else {
        NodePhase::Pending
    };

    SolanaNodeStatus {
        phase: Some(phase),
        slot_height: None,
        healthy: Some(healthy),
        slots_behind: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetStatus};
    use k8s_openapi::api::core::v1::Service;

    fn make_statefulset_status(
        ready: Option<i32>,
        current: Option<i32>,
        observed_generation: Option<i64>,
    ) -> StatefulSetStatus {
        StatefulSetStatus {
            ready_replicas: ready,
            current_replicas: current,
            observed_generation,
            ..Default::default()
        }
    }

    // ── observe_status ────────────────────────────────────────────────────────

    #[test]
    fn observe_status_pending_when_no_statefulset() {
        let status = observe_status(None, Some(&Service::default()), 1);
        assert_eq!(status.phase, Some(NodePhase::Pending));
        assert_eq!(status.healthy, Some(false));
    }

    #[test]
    fn observe_status_pending_when_no_service() {
        let status = observe_status(Some(&StatefulSet::default()), None, 1);
        assert_eq!(status.phase, Some(NodePhase::Pending));
        assert_eq!(status.healthy, Some(false));
    }

    #[test]
    fn observe_status_pending_when_nothing() {
        let status = observe_status(None, None, 1);
        assert_eq!(status.phase, Some(NodePhase::Pending));
        assert_eq!(status.healthy, Some(false));
    }

    // ── phase_from_statefulset_status ─────────────────────────────────────────

    #[test]
    fn phase_running_when_ready_meets_desired() {
        let sts_status = make_statefulset_status(Some(2), Some(2), Some(1));
        let result = phase_from_statefulset_status(&sts_status, 2);
        assert_eq!(result.phase, Some(NodePhase::Running));
        assert_eq!(result.healthy, Some(true));
    }

    #[test]
    fn phase_initializing_when_current_but_not_ready() {
        let sts_status = make_statefulset_status(Some(0), Some(1), Some(1));
        let result = phase_from_statefulset_status(&sts_status, 2);
        assert_eq!(result.phase, Some(NodePhase::Initializing));
        assert_eq!(result.healthy, Some(false));
    }

    #[test]
    fn phase_initializing_when_observed_generation_set() {
        // current=0 but observed_generation is Some → StatefulSet was seen by controller
        let sts_status = make_statefulset_status(Some(0), Some(0), Some(1));
        let result = phase_from_statefulset_status(&sts_status, 1);
        assert_eq!(result.phase, Some(NodePhase::Initializing));
        assert_eq!(result.healthy, Some(false));
    }

    #[test]
    fn phase_pending_when_no_current_replicas_and_no_generation() {
        let sts_status = make_statefulset_status(None, None, None);
        let result = phase_from_statefulset_status(&sts_status, 1);
        assert_eq!(result.phase, Some(NodePhase::Pending));
        assert_eq!(result.healthy, Some(false));
    }

    #[test]
    fn phase_unhealthy_when_desired_is_zero() {
        // desired_replicas=0 makes healthy always false even if ready=0
        let sts_status = make_statefulset_status(Some(0), Some(0), None);
        let result = phase_from_statefulset_status(&sts_status, 0);
        assert_eq!(result.healthy, Some(false));
    }

    // ── validate_external_cluster_config ─────────────────────────────────────

    #[test]
    fn validate_add_worker_ok_with_cluster_name() {
        use crate::crds::ExternalClusterSpec;
        let ext = Some(ExternalClusterSpec {
            provider: crate::crds::Provider::Cherry,
            mode: ExternalClusterMode::AddWorkerToExistingCluster,
            region_preferences: vec!["lt-siauliai".to_string()],
            existing_cluster_name: Some("prod-cluster".to_string()),
            existing_cluster_endpoint: None,
            create_k8s_cluster: false,
            ssh_keys: vec![],
        });
        let result = validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_add_worker_ok_with_cluster_endpoint() {
        use crate::crds::ExternalClusterSpec;
        let ext = Some(ExternalClusterSpec {
            provider: crate::crds::Provider::Latitude,
            mode: ExternalClusterMode::AddWorkerToExistingCluster,
            region_preferences: vec!["us-dal".to_string()],
            existing_cluster_name: None,
            existing_cluster_endpoint: Some("https://10.0.0.1:6443".to_string()),
            create_k8s_cluster: false,
            ssh_keys: vec![],
        });
        let result = validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_add_worker_errors_without_cluster_info() {
        use crate::crds::ExternalClusterSpec;
        let ext = Some(ExternalClusterSpec {
            provider: crate::crds::Provider::Cherry,
            mode: ExternalClusterMode::AddWorkerToExistingCluster,
            region_preferences: vec!["lt-siauliai".to_string()],
            existing_cluster_name: None,
            existing_cluster_endpoint: None,
            create_k8s_cluster: false,
            ssh_keys: vec![],
        });
        let result = validate_external_cluster_config(&ext, &ExternalClusterMode::AddWorkerToExistingCluster);
        assert!(matches!(result, Err(ControllerError::InvalidConfig(_))));
        if let Err(ControllerError::InvalidConfig(msg)) = result {
            assert!(msg.contains("existing_cluster_name"));
        }
    }

    #[test]
    fn validate_provision_new_cluster_always_ok() {
        use crate::crds::ExternalClusterSpec;
        // ProvisionNewCluster has no required fields beyond the spec itself
        let ext = Some(ExternalClusterSpec {
            provider: crate::crds::Provider::Ovh,
            mode: ExternalClusterMode::ProvisionNewCluster,
            region_preferences: vec![],
            existing_cluster_name: None,
            existing_cluster_endpoint: None,
            create_k8s_cluster: true,
            ssh_keys: vec![],
        });
        let result = validate_external_cluster_config(&ext, &ExternalClusterMode::ProvisionNewCluster);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_none_ext_always_ok() {
        // Legacy path: external_cluster not set → no validation
        let result = validate_external_cluster_config(
            &None,
            &ExternalClusterMode::AddWorkerToExistingCluster,
        );
        assert!(result.is_ok());
    }

    // ── build_provider env var validation ─────────────────────────────────────

    #[test]
    fn build_provider_cherry_errors_without_api_key() {
        // Ensure CHERRY_API_KEY is not set so we get the expected error
        std::env::remove_var("CHERRY_API_KEY");
        let result = build_provider(&CrdProvider::Cherry);
        assert!(matches!(result, Err(ControllerError::ProvisionError(_))));
        if let Err(ControllerError::ProvisionError(msg)) = result {
            assert!(msg.contains("CHERRY_API_KEY"));
        }
    }

    #[test]
    fn build_provider_latitude_errors_without_api_key() {
        std::env::remove_var("LATITUDE_API_KEY");
        let result = build_provider(&CrdProvider::Latitude);
        assert!(matches!(result, Err(ControllerError::ProvisionError(_))));
        if let Err(ControllerError::ProvisionError(msg)) = result {
            assert!(msg.contains("LATITUDE_API_KEY"));
        }
    }

    #[test]
    fn build_provider_ovh_errors_without_consumer_key() {
        // Set app key and secret but leave consumer key unset
        std::env::set_var("OVH_APPLICATION_KEY", "dummy_key");
        std::env::set_var("OVH_APPLICATION_SECRET", "dummy_secret");
        std::env::remove_var("OVH_CONSUMER_KEY");
        let result = build_provider(&CrdProvider::Ovh);
        assert!(matches!(result, Err(ControllerError::ProvisionError(_))));
        if let Err(ControllerError::ProvisionError(msg)) = result {
            assert!(msg.contains("OVH_CONSUMER_KEY"));
        }
        // Cleanup
        std::env::remove_var("OVH_APPLICATION_KEY");
        std::env::remove_var("OVH_APPLICATION_SECRET");
    }
}
