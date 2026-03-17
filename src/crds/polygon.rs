use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// PolygonNodeSpec defines the desired state of a Polygon PoS node
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
#[kube(
    group = "blockchain.5dlabs.io",
    version = "v1alpha1",
    kind = "PolygonNode",
    namespaced,
    status = "PolygonNodeStatus",
    printcolumn = r#"{"name": "Type", "type": "string", "jsonPath": ".spec.nodeType"}"#,
    printcolumn = r#"{"name": "Network", "type": "string", "jsonPath": ".spec.network"}"#,
    printcolumn = r#"{"name": "Phase", "type": "string", "jsonPath": ".status.phase"}"#,
    printcolumn = r#"{"name": "Healthy", "type": "boolean", "jsonPath": ".status.healthy"}"#,
    printcolumn = r#"{"name": "Age", "type": "date", "jsonPath": ".metadata.creationTimestamp"}"#
)]
pub struct PolygonNodeSpec {
    /// Type of Polygon node
    #[serde(default = "default_node_type")]
    pub node_type: PolygonNodeType,

    /// Polygon network
    #[serde(default = "default_network")]
    pub network: PolygonNetwork,

    /// Deployment target
    #[serde(default = "default_deployment_target")]
    pub deployment_target: DeploymentTarget,

    /// Bare-metal configuration (required when deployment_target is BareMetal)
    pub bare_metal: Option<BareMetalConfig>,

    /// Compute resources (auto-sized from network/nodeType if omitted)
    pub resources: Option<NodeResources>,

    /// Heimdall consensus layer configuration
    #[serde(default)]
    pub heimdall: HeimdallConfig,

    /// Bor execution layer configuration (for Full/Sentry nodes)
    pub bor: Option<BorConfig>,

    /// Erigon execution layer configuration (for Archive nodes)
    pub erigon: Option<ErigonConfig>,

    /// Storage configuration (auto-sized from network/nodeType if omitted)
    pub storage: Option<StorageConfig>,

    /// Enable Prometheus metrics endpoints
    #[serde(default = "default_enable_metrics")]
    pub enable_metrics: bool,
}

fn default_node_type() -> PolygonNodeType { PolygonNodeType::Full }
fn default_network() -> PolygonNetwork { PolygonNetwork::Mainnet }
fn default_deployment_target() -> DeploymentTarget { DeploymentTarget::InCluster }
fn default_enable_metrics() -> bool { true }

/// Type of Polygon node
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PolygonNodeType {
    Full,
    Archive,
    Sentry,
}

/// Polygon network
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PolygonNetwork {
    Mainnet,
    Amoy,
}

/// Deployment target for the node
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentTarget {
    InCluster,
    BareMetal,
}

/// Bare-metal server configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BareMetalConfig {
    /// Provider name (e.g., "ovh", "cherry", "latitude")
    pub provider: String,

    /// Server ID or reference
    pub server_id: Option<String>,

    /// Server plan
    pub plan: Option<String>,

    /// Deployment region
    pub region: Option<String>,
}

/// Compute resource requirements
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeResources {
    /// CPU request
    pub cpu_request: Option<String>,

    /// Memory request
    pub memory_request: Option<String>,

    /// CPU limit
    pub cpu_limit: Option<String>,

    /// Memory limit
    pub memory_limit: Option<String>,
}

/// Heimdall v2 consensus layer configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeimdallConfig {
    /// Container image for Heimdall
    #[serde(default = "default_heimdall_image")]
    pub image: String,

    /// P2P seed nodes
    pub seeds: Option<Vec<String>>,

    /// P2P listen port
    #[serde(default = "default_heimdall_p2p_port")]
    pub p2p_port: i32,

    /// RPC port (CometBFT RPC)
    #[serde(default = "default_heimdall_rpc_port")]
    pub rpc_port: i32,

    /// REST API port (used by Bor)
    #[serde(default = "default_heimdall_rest_port")]
    pub rest_port: i32,

    /// Prometheus metrics port
    #[serde(default = "default_heimdall_metrics_port")]
    pub metrics_port: i32,

    /// URL to download genesis.json (mainnet genesis is ~3GB)
    pub genesis_url: Option<String>,

    /// Extra command-line arguments
    pub extra_args: Option<Vec<String>>,
}

impl Default for HeimdallConfig {
    fn default() -> Self {
        Self {
            image: default_heimdall_image(),
            seeds: None,
            p2p_port: default_heimdall_p2p_port(),
            rpc_port: default_heimdall_rpc_port(),
            rest_port: default_heimdall_rest_port(),
            metrics_port: default_heimdall_metrics_port(),
            genesis_url: None,
            extra_args: None,
        }
    }
}

fn default_heimdall_image() -> String { "0xpolygon/heimdall-v2:0.6.0".to_string() }
fn default_heimdall_p2p_port() -> i32 { 26656 }
fn default_heimdall_rpc_port() -> i32 { 26657 }
fn default_heimdall_rest_port() -> i32 { 1317 }
fn default_heimdall_metrics_port() -> i32 { 26660 }

/// Bor execution layer configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BorConfig {
    /// Container image for Bor
    #[serde(default = "default_bor_image")]
    pub image: String,

    /// Bootnode ENR addresses
    pub bootnodes: Option<Vec<String>>,

    /// P2P listen port
    #[serde(default = "default_bor_p2p_port")]
    pub p2p_port: i32,

    /// HTTP RPC port
    #[serde(default = "default_bor_http_port")]
    pub http_port: i32,

    /// WebSocket RPC port
    #[serde(default = "default_bor_ws_port")]
    pub ws_port: i32,

    /// Prometheus metrics port
    #[serde(default = "default_bor_metrics_port")]
    pub metrics_port: i32,

    /// Enable HTTP RPC
    #[serde(default = "default_bor_http_enabled")]
    pub http_enabled: bool,

    /// HTTP RPC API namespaces
    #[serde(default = "default_bor_http_api")]
    pub http_api: String,

    /// Enable WebSocket RPC
    #[serde(default = "default_bor_ws_enabled")]
    pub ws_enabled: bool,

    /// Maximum number of peers
    #[serde(default = "default_bor_max_peers")]
    pub max_peers: i32,

    /// Extra command-line arguments
    pub extra_args: Option<Vec<String>>,
}

fn default_bor_image() -> String { "0xpolygon/bor:2.6.3".to_string() }
fn default_bor_p2p_port() -> i32 { 30303 }
fn default_bor_http_port() -> i32 { 8545 }
fn default_bor_ws_port() -> i32 { 8546 }
fn default_bor_metrics_port() -> i32 { 9091 }
fn default_bor_http_enabled() -> bool { true }
fn default_bor_http_api() -> String { "eth,net,web3,txpool,bor".to_string() }
fn default_bor_ws_enabled() -> bool { false }
fn default_bor_max_peers() -> i32 { 50 }

/// Erigon execution layer configuration (archive nodes)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErigonConfig {
    /// Container image for Erigon
    #[serde(default = "default_erigon_image")]
    pub image: String,

    /// P2P listen port
    #[serde(default = "default_erigon_p2p_port")]
    pub p2p_port: i32,

    /// HTTP RPC port
    #[serde(default = "default_erigon_http_port")]
    pub http_port: i32,

    /// Prometheus metrics port
    #[serde(default = "default_erigon_metrics_port")]
    pub metrics_port: i32,

    /// Prune mode
    #[serde(default = "default_erigon_prune_mode")]
    pub prune_mode: String,

    /// Chain identifier (derived from network if not set)
    pub chain: Option<String>,

    /// Extra command-line arguments
    pub extra_args: Option<Vec<String>>,
}

fn default_erigon_image() -> String { "erigontech/erigon:v3.2.2".to_string() }
fn default_erigon_p2p_port() -> i32 { 30303 }
fn default_erigon_http_port() -> i32 { 8545 }
fn default_erigon_metrics_port() -> i32 { 9091 }
fn default_erigon_prune_mode() -> String { "archive".to_string() }

/// Storage configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    /// Kubernetes StorageClass name
    pub storage_class: Option<String>,

    /// Heimdall data volume size
    pub heimdall_size: Option<String>,

    /// Execution layer data volume size (Bor or Erigon)
    pub execution_size: Option<String>,
}

/// Recommended sizing for a given (network, node_type) combination
pub struct RecommendedSizing {
    pub heimdall_size: &'static str,
    pub execution_size: &'static str,
    pub storage_class: &'static str,
    pub cpu_request: &'static str,
    pub memory_request: &'static str,
    pub cpu_limit: &'static str,
    pub memory_limit: &'static str,
}

impl RecommendedSizing {
    pub fn for_network_and_type(network: &PolygonNetwork, node_type: &PolygonNodeType) -> Self {
        match (network, node_type) {
            (PolygonNetwork::Mainnet, PolygonNodeType::Full | PolygonNodeType::Sentry) => Self {
                heimdall_size: "1Ti",
                execution_size: "8Ti",
                storage_class: "csi-cinder-high-speed",
                cpu_request: "16",
                memory_request: "64Gi",
                cpu_limit: "32",
                memory_limit: "128Gi",
            },
            (PolygonNetwork::Mainnet, PolygonNodeType::Archive) => Self {
                heimdall_size: "1Ti",
                execution_size: "16Ti",
                storage_class: "csi-cinder-high-speed",
                cpu_request: "16",
                memory_request: "128Gi",
                cpu_limit: "32",
                memory_limit: "128Gi",
            },
            (PolygonNetwork::Amoy, PolygonNodeType::Full | PolygonNodeType::Sentry) => Self {
                heimdall_size: "50Gi",
                execution_size: "500Gi",
                storage_class: "csi-cinder-high-speed",
                cpu_request: "4",
                memory_request: "16Gi",
                cpu_limit: "8",
                memory_limit: "32Gi",
            },
            (PolygonNetwork::Amoy, PolygonNodeType::Archive) => Self {
                heimdall_size: "50Gi",
                execution_size: "1Ti",
                storage_class: "csi-cinder-high-speed",
                cpu_request: "4",
                memory_request: "32Gi",
                cpu_limit: "8",
                memory_limit: "32Gi",
            },
        }
    }
}

impl PolygonNodeSpec {
    /// Returns effective StorageConfig, merging user overrides with recommended defaults.
    pub fn effective_storage(&self) -> StorageConfig {
        let defaults = RecommendedSizing::for_network_and_type(&self.network, &self.node_type);
        match &self.storage {
            Some(user) => StorageConfig {
                storage_class: Some(user.storage_class.clone()
                    .unwrap_or_else(|| defaults.storage_class.to_string())),
                heimdall_size: Some(user.heimdall_size.clone()
                    .unwrap_or_else(|| defaults.heimdall_size.to_string())),
                execution_size: Some(user.execution_size.clone()
                    .unwrap_or_else(|| defaults.execution_size.to_string())),
            },
            None => StorageConfig {
                storage_class: Some(defaults.storage_class.to_string()),
                heimdall_size: Some(defaults.heimdall_size.to_string()),
                execution_size: Some(defaults.execution_size.to_string()),
            },
        }
    }

    /// Returns effective NodeResources, merging user overrides with recommended defaults.
    pub fn effective_resources(&self) -> NodeResources {
        let defaults = RecommendedSizing::for_network_and_type(&self.network, &self.node_type);
        match &self.resources {
            Some(user) => NodeResources {
                cpu_request: Some(user.cpu_request.clone()
                    .unwrap_or_else(|| defaults.cpu_request.to_string())),
                memory_request: Some(user.memory_request.clone()
                    .unwrap_or_else(|| defaults.memory_request.to_string())),
                cpu_limit: Some(user.cpu_limit.clone()
                    .unwrap_or_else(|| defaults.cpu_limit.to_string())),
                memory_limit: Some(user.memory_limit.clone()
                    .unwrap_or_else(|| defaults.memory_limit.to_string())),
            },
            None => NodeResources {
                cpu_request: Some(defaults.cpu_request.to_string()),
                memory_request: Some(defaults.memory_request.to_string()),
                cpu_limit: Some(defaults.cpu_limit.to_string()),
                memory_limit: Some(defaults.memory_limit.to_string()),
            },
        }
    }
}

/// PolygonNodeStatus defines the observed state of a PolygonNode
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PolygonNodeStatus {
    /// Current phase of the node
    pub phase: Option<PolygonNodePhase>,

    /// Whether Heimdall is synced
    pub heimdall_synced: Option<bool>,

    /// Heimdall block height
    pub heimdall_height: Option<i64>,

    /// Whether execution layer is synced
    pub execution_synced: Option<bool>,

    /// Execution layer block number
    pub execution_block: Option<i64>,

    /// Whether the node is healthy
    pub healthy: Option<bool>,

    /// Number of connected peers
    pub peers: Option<i32>,

    /// Human-readable status message
    pub message: Option<String>,
}

/// Phase of the Polygon node lifecycle
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PolygonNodePhase {
    Pending,
    Initializing,
    HeimdallSyncing,
    ExecutionSyncing,
    Running,
    Error,
}
