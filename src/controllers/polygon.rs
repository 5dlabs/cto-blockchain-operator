use crate::crds::polygon::*;
use crate::crds::PolygonNode;
use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec};
use k8s_openapi::api::core::v1::{
    ConfigMap, Container, ContainerPort, ExecAction, HTTPGetAction, Probe, Service,
    ServicePort, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{ObjectMeta, Patch, PatchParams};
use kube::{Api, Client, ResourceExt};
use std::collections::BTreeMap;
use thiserror::Error;
use tracing::*;

#[derive(Error, Debug)]
pub enum PolygonControllerError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Failed to create Kubernetes resources: {0}")]
    K8sError(String),
    #[error("Kubernetes API error: {0}")]
    Kubernetes(#[from] kube::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("TOML serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),
}

pub struct PolygonController {
    client: Client,
}

impl PolygonController {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn reconcile(&self, node: &PolygonNode) -> Result<PolygonNodeStatus, PolygonControllerError> {
        let name = node.name_any();
        let namespace = node.namespace().unwrap_or_else(|| "default".to_string());

        info!("Reconciling PolygonNode {} in namespace {}", name, namespace);

        // Step 1: Validate spec
        self.validate_spec(&node.spec)?;

        // Step 2: Ensure ConfigMaps
        self.ensure_configmaps(node, &name, &namespace).await?;

        // Step 3: Ensure StatefulSet
        self.ensure_statefulset(node, &name, &namespace).await?;

        // Step 4: Ensure Services
        self.ensure_services(node, &name, &namespace).await?;

        // Step 5: Check status and return
        let status = self.check_status(node, &name, &namespace).await?;

        info!("PolygonNode {} reconciled, phase: {:?}", name, status.phase);

        Ok(status)
    }

    fn validate_spec(&self, spec: &PolygonNodeSpec) -> Result<(), PolygonControllerError> {
        match spec.node_type {
            PolygonNodeType::Full | PolygonNodeType::Sentry => {
                if spec.bor.is_none() {
                    return Err(PolygonControllerError::ValidationError(
                        format!("{:?} nodes require bor configuration", spec.node_type),
                    ));
                }
                if spec.erigon.is_some() {
                    return Err(PolygonControllerError::ValidationError(
                        format!("{:?} nodes should not have erigon configuration", spec.node_type),
                    ));
                }
            }
            PolygonNodeType::Archive => {
                if spec.erigon.is_none() {
                    return Err(PolygonControllerError::ValidationError(
                        "Archive nodes require erigon configuration".to_string(),
                    ));
                }
                if spec.bor.is_some() {
                    return Err(PolygonControllerError::ValidationError(
                        "Archive nodes should not have bor configuration".to_string(),
                    ));
                }
            }
        }

        if spec.deployment_target == DeploymentTarget::BareMetal && spec.bare_metal.is_none() {
            return Err(PolygonControllerError::ValidationError(
                "BareMetal deployment target requires bare_metal configuration".to_string(),
            ));
        }

        Ok(())
    }

    async fn ensure_configmaps(
        &self,
        node: &PolygonNode,
        name: &str,
        namespace: &str,
    ) -> Result<(), PolygonControllerError> {
        let configmaps: Api<ConfigMap> = Api::namespaced(self.client.clone(), namespace);
        let owner_ref = self.owner_reference(node);

        // Heimdall config
        let heimdall_config = self.generate_heimdall_config(&node.spec);
        let heimdall_app_config = self.generate_heimdall_app_config(&node.spec);

        let heimdall_cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some(format!("{}-heimdall-config", name)),
                namespace: Some(namespace.to_string()),
                owner_references: Some(vec![owner_ref.clone()]),
                ..Default::default()
            },
            data: Some(BTreeMap::from([
                ("config.toml".to_string(), heimdall_config),
                ("app.toml".to_string(), heimdall_app_config),
            ])),
            ..Default::default()
        };

        configmaps
            .patch(
                &format!("{}-heimdall-config", name),
                &PatchParams::apply("cto-operator"),
                &Patch::Apply(heimdall_cm),
            )
            .await?;

        // Bor config (if applicable)
        if let Some(ref bor) = node.spec.bor {
            let bor_config = self.generate_bor_config(&node.spec, bor);

            let bor_cm = ConfigMap {
                metadata: ObjectMeta {
                    name: Some(format!("{}-bor-config", name)),
                    namespace: Some(namespace.to_string()),
                    owner_references: Some(vec![owner_ref.clone()]),
                    ..Default::default()
                },
                data: Some(BTreeMap::from([
                    ("config.toml".to_string(), bor_config),
                ])),
                ..Default::default()
            };

            configmaps
                .patch(
                    &format!("{}-bor-config", name),
                    &PatchParams::apply("cto-operator"),
                    &Patch::Apply(bor_cm),
                )
                .await?;
        }

        info!("ConfigMaps ensured for PolygonNode {}", name);
        Ok(())
    }

    fn generate_heimdall_config(&self, spec: &PolygonNodeSpec) -> String {
        let seeds = spec
            .heimdall
            .seeds
            .as_ref()
            .map(|s| s.join(","))
            .unwrap_or_default();

        let chain_id = match spec.network {
            PolygonNetwork::Mainnet => "heimdall-137",
            PolygonNetwork::Amoy => "heimdall-80002",
        };

        format!(
            r#"# Heimdall v2 Configuration
# Auto-generated by CTO Blockchain Operator

moniker = "cto-polygon-node"
chain_id = "{chain_id}"

[p2p]
seeds = "{seeds}"
laddr = "tcp://0.0.0.0:{p2p_port}"
max_num_inbound_peers = 40
max_num_outbound_peers = 10
persistent_peers = ""

[rpc]
laddr = "tcp://0.0.0.0:{rpc_port}"

[instrumentation]
prometheus = true
prometheus_listen_addr = ":{metrics_port}"
"#,
            chain_id = chain_id,
            seeds = seeds,
            p2p_port = spec.heimdall.p2p_port,
            rpc_port = spec.heimdall.rpc_port,
            metrics_port = spec.heimdall.metrics_port,
        )
    }

    fn generate_heimdall_app_config(&self, spec: &PolygonNodeSpec) -> String {
        let eth_rpc_url = match spec.network {
            PolygonNetwork::Mainnet => "https://eth-mainnet.public.blastapi.io",
            PolygonNetwork::Amoy => "https://eth-sepolia.public.blastapi.io",
        };

        let bor_rpc_url = match spec.node_type {
            PolygonNodeType::Archive => format!("http://localhost:{}", spec.erigon.as_ref().map(|e| e.http_port).unwrap_or(8545)),
            _ => format!("http://localhost:{}", spec.bor.as_ref().map(|b| b.http_port).unwrap_or(8545)),
        };

        format!(
            r#"# Heimdall v2 App Configuration
# Auto-generated by CTO Blockchain Operator

[api]
enable = true
address = "tcp://0.0.0.0:{rest_port}"

[grpc]
enable = false

[telemetry]
enabled = true

[heimdall]
eth_rpc_url = "{eth_rpc_url}"
bor_rpc_url = "{bor_rpc_url}"
"#,
            rest_port = spec.heimdall.rest_port,
            eth_rpc_url = eth_rpc_url,
            bor_rpc_url = bor_rpc_url,
        )
    }

    fn generate_bor_config(&self, spec: &PolygonNodeSpec, bor: &BorConfig) -> String {
        let bootnodes = bor
            .bootnodes
            .as_ref()
            .map(|b| b.join(","))
            .unwrap_or_default();

        let chain = match spec.network {
            PolygonNetwork::Mainnet => "mainnet",
            PolygonNetwork::Amoy => "amoy",
        };

        let http_api = &bor.http_api;

        format!(
            r#"# Bor Configuration
# Auto-generated by CTO Blockchain Operator

chain = "{chain}"
datadir = "/var/lib/bor/data"

[p2p]
  maxpeers = {max_peers}
  port = {p2p_port}
  [p2p.discovery]
    bootnodes = ["{bootnodes}"]

[heimdall]
  url = "http://localhost:{heimdall_rest_port}"

[jsonrpc]
  [jsonrpc.http]
    enabled = {http_enabled}
    host = "0.0.0.0"
    port = {http_port}
    api = ["{http_api_list}"]
  [jsonrpc.ws]
    enabled = {ws_enabled}
    host = "0.0.0.0"
    port = {ws_port}

[telemetry]
  metrics = true
  prometheus-addr = ":{metrics_port}"
"#,
            chain = chain,
            max_peers = bor.max_peers,
            p2p_port = bor.p2p_port,
            bootnodes = bootnodes,
            heimdall_rest_port = spec.heimdall.rest_port,
            http_enabled = bor.http_enabled,
            http_port = bor.http_port,
            http_api_list = http_api.split(',').collect::<Vec<_>>().join("\", \""),
            ws_enabled = bor.ws_enabled,
            ws_port = bor.ws_port,
            metrics_port = bor.metrics_port,
        )
    }

    async fn ensure_statefulset(
        &self,
        node: &PolygonNode,
        name: &str,
        namespace: &str,
    ) -> Result<(), PolygonControllerError> {
        let statefulsets: Api<StatefulSet> = Api::namespaced(self.client.clone(), namespace);
        let owner_ref = self.owner_reference(node);
        let spec = &node.spec;

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "polygon-node".to_string());
        labels.insert("node.blockchain.5dlabs.io/name".to_string(), name.to_string());
        labels.insert("node.blockchain.5dlabs.io/type".to_string(), format!("{:?}", spec.node_type).to_lowercase());
        labels.insert("node.blockchain.5dlabs.io/network".to_string(), format!("{:?}", spec.network).to_lowercase());

        // Build containers
        let mut containers = vec![self.heimdall_container(spec)];
        let mut init_containers = Vec::new();

        // Genesis init container for mainnet (genesis.json is ~3GB)
        if let Some(ref genesis_url) = spec.heimdall.genesis_url {
            init_containers.push(Container {
                name: "download-genesis".to_string(),
                image: Some("curlimages/curl:latest".to_string()),
                command: Some(vec!["sh".to_string(), "-c".to_string()]),
                args: Some(vec![format!(
                    "if [ ! -f /heimdall-data/config/genesis.json ]; then mkdir -p /heimdall-data/config && curl -L -o /heimdall-data/config/genesis.json '{}'; fi",
                    genesis_url
                )]),
                volume_mounts: Some(vec![VolumeMount {
                    name: "heimdall-data".to_string(),
                    mount_path: "/heimdall-data".to_string(),
                    ..Default::default()
                }]),
                ..Default::default()
            });
        }

        match spec.node_type {
            PolygonNodeType::Full | PolygonNodeType::Sentry => {
                containers.push(self.bor_container(spec, spec.bor.as_ref().unwrap()));
            }
            PolygonNodeType::Archive => {
                containers.push(self.erigon_container(spec, spec.erigon.as_ref().unwrap()));
            }
        }

        // Volume claim templates
        let heimdall_pvc = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "heimdall-data"
            },
            "spec": {
                "accessModes": ["ReadWriteOnce"],
                "storageClassName": spec.storage.storage_class,
                "resources": {
                    "requests": {
                        "storage": spec.storage.heimdall_size
                    }
                }
            }
        }))?;

        let execution_pvc = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "execution-data"
            },
            "spec": {
                "accessModes": ["ReadWriteOnce"],
                "storageClassName": spec.storage.storage_class,
                "resources": {
                    "requests": {
                        "storage": spec.storage.execution_size
                    }
                }
            }
        }))?;

        // ConfigMap volumes
        let mut volumes = vec![Volume {
            name: "heimdall-config".to_string(),
            config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                name: Some(format!("{}-heimdall-config", name)),
                ..Default::default()
            }),
            ..Default::default()
        }];

        if spec.bor.is_some() {
            volumes.push(Volume {
                name: "bor-config".to_string(),
                config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                    name: Some(format!("{}-bor-config", name)),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        let resource_requirements = serde_json::from_value(serde_json::json!({
            "requests": {
                "cpu": spec.resources.cpu_request,
                "memory": spec.resources.memory_request
            },
            "limits": {
                "cpu": spec.resources.cpu_limit.as_deref().unwrap_or(&spec.resources.cpu_request),
                "memory": spec.resources.memory_limit.as_deref().unwrap_or(&spec.resources.memory_request)
            }
        }))?;

        // Apply resource requirements to the execution container (main workload)
        if let Some(exec_container) = containers.last_mut() {
            exec_container.resources = Some(resource_requirements);
        }

        let sts = StatefulSet {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels.clone()),
                owner_references: Some(vec![owner_ref]),
                ..Default::default()
            },
            spec: Some(StatefulSetSpec {
                replicas: Some(1),
                selector: serde_json::from_value(serde_json::json!({
                    "matchLabels": {
                        "app": "polygon-node",
                        "node.blockchain.5dlabs.io/name": name
                    }
                }))?,
                service_name: format!("{}-headless", name),
                template: serde_json::from_value(serde_json::json!({
                    "metadata": {
                        "labels": labels
                    },
                    "spec": {
                        "initContainers": init_containers,
                        "containers": containers,
                        "volumes": volumes
                    }
                }))?,
                volume_claim_templates: Some(vec![heimdall_pvc, execution_pvc]),
                ..Default::default()
            }),
            ..Default::default()
        };

        statefulsets
            .patch(name, &PatchParams::apply("cto-operator"), &Patch::Apply(sts))
            .await?;

        info!("StatefulSet ensured for PolygonNode {}", name);
        Ok(())
    }

    fn heimdall_container(&self, spec: &PolygonNodeSpec) -> Container {
        let mut args = vec![
            "start".to_string(),
            "--home=/heimdall-data".to_string(),
        ];

        if let Some(ref extra) = spec.heimdall.extra_args {
            args.extend(extra.clone());
        }

        Container {
            name: "heimdall".to_string(),
            image: Some(spec.heimdall.image.clone()),
            args: Some(args),
            ports: Some(vec![
                ContainerPort {
                    name: Some("p2p".to_string()),
                    container_port: spec.heimdall.p2p_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("rpc".to_string()),
                    container_port: spec.heimdall.rpc_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("rest".to_string()),
                    container_port: spec.heimdall.rest_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("metrics".to_string()),
                    container_port: spec.heimdall.metrics_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
            ]),
            readiness_probe: Some(Probe {
                http_get: Some(HTTPGetAction {
                    path: Some("/node_info".to_string()),
                    port: IntOrString::Int(spec.heimdall.rest_port),
                    ..Default::default()
                }),
                initial_delay_seconds: Some(30),
                period_seconds: Some(10),
                ..Default::default()
            }),
            liveness_probe: Some(Probe {
                http_get: Some(HTTPGetAction {
                    path: Some("/node_info".to_string()),
                    port: IntOrString::Int(spec.heimdall.rest_port),
                    ..Default::default()
                }),
                initial_delay_seconds: Some(60),
                period_seconds: Some(30),
                ..Default::default()
            }),
            volume_mounts: Some(vec![
                VolumeMount {
                    name: "heimdall-data".to_string(),
                    mount_path: "/heimdall-data".to_string(),
                    ..Default::default()
                },
                VolumeMount {
                    name: "heimdall-config".to_string(),
                    mount_path: "/heimdall-data/config/config.toml".to_string(),
                    sub_path: Some("config.toml".to_string()),
                    ..Default::default()
                },
                VolumeMount {
                    name: "heimdall-config".to_string(),
                    mount_path: "/heimdall-data/config/app.toml".to_string(),
                    sub_path: Some("app.toml".to_string()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }
    }

    fn bor_container(&self, spec: &PolygonNodeSpec, bor: &BorConfig) -> Container {
        let chain = match spec.network {
            PolygonNetwork::Mainnet => "mainnet",
            PolygonNetwork::Amoy => "amoy",
        };

        let mut args = vec![
            "server".to_string(),
            format!("--chain={}", chain),
            "--datadir=/bor-data".to_string(),
            format!("--bor.heimdall=http://localhost:{}", spec.heimdall.rest_port),
            format!("--maxpeers={}", bor.max_peers),
            format!("--port={}", bor.p2p_port),
            "--config=/bor-config/config.toml".to_string(),
        ];

        if bor.http_enabled {
            args.push("--http".to_string());
            args.push("--http.addr=0.0.0.0".to_string());
            args.push(format!("--http.port={}", bor.http_port));
            args.push(format!("--http.api={}", bor.http_api));
        }

        if bor.ws_enabled {
            args.push("--ws".to_string());
            args.push("--ws.addr=0.0.0.0".to_string());
            args.push(format!("--ws.port={}", bor.ws_port));
        }

        if spec.enable_metrics {
            args.push("--metrics".to_string());
            args.push(format!("--metrics.addr=0.0.0.0:{}", bor.metrics_port));
        }

        if let Some(ref bootnodes) = bor.bootnodes {
            args.push(format!("--bootnodes={}", bootnodes.join(",")));
        }

        if let Some(ref extra) = bor.extra_args {
            args.extend(extra.clone());
        }

        Container {
            name: "bor".to_string(),
            image: Some(bor.image.clone()),
            args: Some(args),
            ports: Some(vec![
                ContainerPort {
                    name: Some("p2p".to_string()),
                    container_port: bor.p2p_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("http-rpc".to_string()),
                    container_port: bor.http_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("ws-rpc".to_string()),
                    container_port: bor.ws_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("metrics".to_string()),
                    container_port: bor.metrics_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
            ]),
            // Wait for Heimdall REST API before starting (up to 10 min)
            startup_probe: Some(Probe {
                exec: Some(ExecAction {
                    command: Some(vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        format!("wget -q -O- http://localhost:{}/node_info > /dev/null 2>&1", spec.heimdall.rest_port),
                    ]),
                }),
                failure_threshold: Some(60),
                period_seconds: Some(10),
                ..Default::default()
            }),
            readiness_probe: Some(Probe {
                exec: Some(ExecAction {
                    command: Some(vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        format!(
                            "wget -q -O- --header='Content-Type: application/json' --post-data='{{\"jsonrpc\":\"2.0\",\"method\":\"eth_syncing\",\"params\":[],\"id\":1}}' http://localhost:{}/",
                            bor.http_port
                        ),
                    ]),
                }),
                initial_delay_seconds: Some(30),
                period_seconds: Some(15),
                ..Default::default()
            }),
            volume_mounts: Some(vec![
                VolumeMount {
                    name: "execution-data".to_string(),
                    mount_path: "/bor-data".to_string(),
                    ..Default::default()
                },
                VolumeMount {
                    name: "bor-config".to_string(),
                    mount_path: "/bor-config/config.toml".to_string(),
                    sub_path: Some("config.toml".to_string()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }
    }

    fn erigon_container(&self, spec: &PolygonNodeSpec, erigon: &ErigonConfig) -> Container {
        let chain = erigon.chain.clone().unwrap_or_else(|| {
            match spec.network {
                PolygonNetwork::Mainnet => "bor-mainnet".to_string(),
                PolygonNetwork::Amoy => "amoy".to_string(),
            }
        });

        let mut args = vec![
            format!("--chain={}", chain),
            "--datadir=/erigon-data".to_string(),
            format!("--bor.heimdall=http://localhost:{}", spec.heimdall.rest_port),
            format!("--port={}", erigon.p2p_port),
            "--http".to_string(),
            "--http.addr=0.0.0.0".to_string(),
            format!("--http.port={}", erigon.http_port),
            "--http.api=eth,erigon,net,web3,txpool,bor".to_string(),
            format!("--prune={}", erigon.prune_mode),
        ];

        if spec.enable_metrics {
            args.push("--metrics".to_string());
            args.push(format!("--metrics.addr=0.0.0.0:{}", erigon.metrics_port));
        }

        if let Some(ref extra) = erigon.extra_args {
            args.extend(extra.clone());
        }

        Container {
            name: "erigon".to_string(),
            image: Some(erigon.image.clone()),
            args: Some(args),
            ports: Some(vec![
                ContainerPort {
                    name: Some("p2p".to_string()),
                    container_port: erigon.p2p_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("http-rpc".to_string()),
                    container_port: erigon.http_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
                ContainerPort {
                    name: Some("metrics".to_string()),
                    container_port: erigon.metrics_port,
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                },
            ]),
            // Wait for Heimdall REST API before starting (up to 10 min)
            startup_probe: Some(Probe {
                exec: Some(ExecAction {
                    command: Some(vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        format!("wget -q -O- http://localhost:{}/node_info > /dev/null 2>&1", spec.heimdall.rest_port),
                    ]),
                }),
                failure_threshold: Some(60),
                period_seconds: Some(10),
                ..Default::default()
            }),
            readiness_probe: Some(Probe {
                exec: Some(ExecAction {
                    command: Some(vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        format!(
                            "wget -q -O- --header='Content-Type: application/json' --post-data='{{\"jsonrpc\":\"2.0\",\"method\":\"eth_syncing\",\"params\":[],\"id\":1}}' http://localhost:{}/",
                            erigon.http_port
                        ),
                    ]),
                }),
                initial_delay_seconds: Some(30),
                period_seconds: Some(15),
                ..Default::default()
            }),
            volume_mounts: Some(vec![VolumeMount {
                name: "execution-data".to_string(),
                mount_path: "/erigon-data".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    async fn ensure_services(
        &self,
        node: &PolygonNode,
        name: &str,
        namespace: &str,
    ) -> Result<(), PolygonControllerError> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        let owner_ref = self.owner_reference(node);
        let spec = &node.spec;

        let selector = BTreeMap::from([
            ("app".to_string(), "polygon-node".to_string()),
            ("node.blockchain.5dlabs.io/name".to_string(), name.to_string()),
        ]);

        // Headless service for StatefulSet
        let headless_svc = Service {
            metadata: ObjectMeta {
                name: Some(format!("{}-headless", name)),
                namespace: Some(namespace.to_string()),
                owner_references: Some(vec![owner_ref.clone()]),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                cluster_ip: Some("None".to_string()),
                selector: Some(selector.clone()),
                ports: Some(vec![
                    ServicePort {
                        name: Some("heimdall-p2p".to_string()),
                        port: spec.heimdall.p2p_port,
                        ..Default::default()
                    },
                    ServicePort {
                        name: Some("heimdall-rpc".to_string()),
                        port: spec.heimdall.rpc_port,
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };

        services
            .patch(
                &format!("{}-headless", name),
                &PatchParams::apply("cto-operator"),
                &Patch::Apply(headless_svc),
            )
            .await?;

        // RPC service
        let mut rpc_ports = vec![
            ServicePort {
                name: Some("heimdall-rest".to_string()),
                port: spec.heimdall.rest_port,
                ..Default::default()
            },
        ];

        let exec_http_port = match spec.node_type {
            PolygonNodeType::Archive => spec.erigon.as_ref().map(|e| e.http_port).unwrap_or(8545),
            _ => spec.bor.as_ref().map(|b| b.http_port).unwrap_or(8545),
        };

        rpc_ports.push(ServicePort {
            name: Some("exec-http".to_string()),
            port: exec_http_port,
            ..Default::default()
        });

        if spec.enable_metrics {
            rpc_ports.push(ServicePort {
                name: Some("heimdall-metrics".to_string()),
                port: spec.heimdall.metrics_port,
                ..Default::default()
            });
            let exec_metrics_port = match spec.node_type {
                PolygonNodeType::Archive => spec.erigon.as_ref().map(|e| e.metrics_port).unwrap_or(9091),
                _ => spec.bor.as_ref().map(|b| b.metrics_port).unwrap_or(9091),
            };
            rpc_ports.push(ServicePort {
                name: Some("exec-metrics".to_string()),
                port: exec_metrics_port,
                ..Default::default()
            });
        }

        let rpc_svc = Service {
            metadata: ObjectMeta {
                name: Some(format!("{}-rpc", name)),
                namespace: Some(namespace.to_string()),
                owner_references: Some(vec![owner_ref]),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(selector),
                ports: Some(rpc_ports),
                ..Default::default()
            }),
            ..Default::default()
        };

        services
            .patch(
                &format!("{}-rpc", name),
                &PatchParams::apply("cto-operator"),
                &Patch::Apply(rpc_svc),
            )
            .await?;

        info!("Services ensured for PolygonNode {}", name);
        Ok(())
    }

    async fn check_status(
        &self,
        _node: &PolygonNode,
        name: &str,
        namespace: &str,
    ) -> Result<PolygonNodeStatus, PolygonControllerError> {
        let pods: Api<k8s_openapi::api::core::v1::Pod> =
            Api::namespaced(self.client.clone(), namespace);

        let pod_name = format!("{}-0", name);
        match pods.get_opt(&pod_name).await? {
            Some(pod) => {
                let phase = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or("Unknown");

                let container_statuses = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.container_statuses.as_ref());

                let all_ready = container_statuses
                    .map(|cs| cs.iter().all(|c| c.ready))
                    .unwrap_or(false);

                let node_phase = match (phase, all_ready) {
                    ("Running", true) => PolygonNodePhase::Running,
                    ("Running", false) => PolygonNodePhase::HeimdallSyncing,
                    ("Pending", _) => PolygonNodePhase::Pending,
                    _ => PolygonNodePhase::Initializing,
                };

                Ok(PolygonNodeStatus {
                    phase: Some(node_phase),
                    healthy: Some(all_ready),
                    message: Some(format!("Pod phase: {}", phase)),
                    ..Default::default()
                })
            }
            None => Ok(PolygonNodeStatus {
                phase: Some(PolygonNodePhase::Pending),
                healthy: Some(false),
                message: Some("Waiting for pod to be created".to_string()),
                ..Default::default()
            }),
        }
    }

    fn owner_reference(&self, node: &PolygonNode) -> OwnerReference {
        OwnerReference {
            api_version: "blockchain.5dlabs.io/v1alpha1".to_string(),
            kind: "PolygonNode".to_string(),
            name: node.name_any(),
            uid: node.metadata.uid.clone().unwrap_or_default(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }
    }
}
