use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Provider for bare-metal server
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Cherry,
    Latitude,
    Ovh,
}

/// Deployment mode: in-cluster (K8s) or external bare-metal
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentMode {
    /// Deploy inside the operator's Kubernetes cluster
    InCluster,
    /// Provision external bare-metal servers (via provider API)
    External,
}

/// Node pool role types
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NodePoolRole {
    /// Solana validator/RPC node
    SolanaRpc,
    /// Support services (QuestDB, PostgreSQL, Redis, Big Balls)
    SupportServices,
}

/// Node pool specification
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct NodePoolSpec {
    /// Role of this node pool
    pub role: NodePoolRole,
    /// Number of nodes in this pool
    #[serde(default = "default_pool_replicas")]
    pub replicas: i32,
    /// Compute resources per node
    pub resources: NodeResources,
    /// Node-specific configuration
    #[serde(default)]
    pub config: Option<NodePoolConfig>,
}

fn default_pool_replicas() -> i32 { 1 }

/// Node pool configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct NodePoolConfig {
    /// For SolanaRpc: node type (validator/rpc/archival)
    #[serde(default)]
    pub node_type: Option<NodeType>,
    /// For SolanaRpc: container image
    #[serde(default)]
    pub image: Option<String>,
    /// For SupportServices: list of services to deploy
    #[serde(default)]
    pub services: Option<Vec<SupportService>>,
}

/// Support services to deploy
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SupportService {
    Questdb,
    Postgres,
    Redis,
    BigBalls,
}

/// External cluster scenario
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalClusterMode {
    /// Provision worker nodes and join an already-existing cluster
    AddWorkerToExistingCluster,
    /// Provision control-plane + workers for a brand new cluster
    ProvisionNewCluster,
}

fn default_external_cluster_mode() -> ExternalClusterMode {
    ExternalClusterMode::AddWorkerToExistingCluster
}

/// External cluster configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ExternalClusterSpec {
    /// Provider for bare-metal servers
    pub provider: Provider,
    /// Scenario selector:
    /// 1) Add worker node(s) to an existing cluster
    /// 2) Provision a brand-new cluster in target region
    #[serde(default = "default_external_cluster_mode")]
    pub mode: ExternalClusterMode,
    /// Preferred regions (in order of priority)
    #[serde(default = "default_regions")]
    pub region_preferences: Vec<String>,
    /// Existing cluster identifier/name (used when mode=AddWorkerToExistingCluster)
    #[serde(default)]
    pub existing_cluster_name: Option<String>,
    /// Existing cluster API endpoint (optional)
    #[serde(default)]
    pub existing_cluster_endpoint: Option<String>,
    /// Whether to bootstrap Kubernetes automatically (used when mode=ProvisionNewCluster)
    #[serde(default)]
    pub create_k8s_cluster: bool,
    /// SSH keys for server access
    #[serde(default)]
    pub ssh_keys: Vec<String>,
}

fn default_regions() -> Vec<String> {
    vec!["nl-ams".to_string(), "lt-siauliai".to_string()]
}

/// SolanaNodeSpec defines the desired state of SolanaNode
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "blockchain.5dlabs.io",
    version = "v1alpha1", 
    kind = "SolanaNode",
    namespaced,
    status = "SolanaNodeStatus",
    derive = "PartialEq",
    finalizer = "solananodes.blockchain.5dlabs.io"
)]
pub struct SolanaNodeSpec {
    /// Deployment mode: in-cluster or external bare-metal
    #[serde(default = "default_deployment_mode")]
    pub deployment_mode: DeploymentMode,
    
    /// Node pools for the cluster (defines roles like solana-rpc, support-services)
    #[serde(default)]
    pub node_pools: Vec<NodePoolSpec>,
    
    /// External cluster configuration (when deployment_mode is External)
    #[serde(default)]
    pub external_cluster: Option<ExternalClusterSpec>,
    
    /// Provider for bare-metal server (legacy, use external_cluster.provider)
    #[serde(default = "default_provider")]
    pub provider: Provider,
    
    /// Region for the bare-metal server (legacy, use external_cluster.region_preferences)
    #[serde(default = "default_region")]
    pub region: String,
    
    /// Number of replicas (legacy)
    #[serde(default = "default_replicas")]
    pub replicas: i32,
    
    /// Type of node (validator, rpc, archival)
    #[serde(default = "default_node_type")]
    pub node_type: NodeType,
    
    /// RPC port
    #[serde(default = "default_rpc_port")]
    pub rpc_port: i32,
    
    /// Gossip port
    #[serde(default = "default_gossip_port")]
    pub gossip_port: i32,
    
    /// Compute resources
    pub resources: NodeResources,
    
    /// Node configuration
    pub config: NodeConfig,
    
    /// Container image
    #[serde(default = "default_image")]
    pub image: String,
    
    /// Enable voting (validator only)
    #[serde(default = "default_enable_voting")]
    pub enable_voting: bool,
    
    /// Identity secret name
    pub identity_secret: String,
    
    /// Known validators to trust
    pub known_validators: Option<Vec<String>>,
    
    /// Entrypoints to connect to
    pub entrypoints: Option<Vec<String>>,
}

fn default_provider() -> Provider { Provider::Cherry }
fn default_deployment_mode() -> DeploymentMode { DeploymentMode::External }
fn default_region() -> String { "nl-ams".to_string() }
fn default_replicas() -> i32 { 1 }
fn default_node_type() -> NodeType { NodeType::Validator }
fn default_rpc_port() -> i32 { 8899 }
fn default_gossip_port() -> i32 { 8001 }
fn default_image() -> String { "anzaxyz/agave:v3.1.9".to_string() }
fn default_enable_voting() -> bool { false }

/// Node type
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Validator,
    Rpc,
    Archival,
}

/// Node resource requirements
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct NodeResources {
    /// CPU request
    #[serde(default = "default_cpu_request")]
    pub cpu_request: String,
    
    /// Memory request
    #[serde(default = "default_memory_request")]
    pub memory_request: String,
    
    /// CPU limit (optional)
    pub cpu_limit: Option<String>,
    
    /// Memory limit (optional)
    pub memory_limit: Option<String>,
}

fn default_cpu_request() -> String { "28".to_string() }
fn default_memory_request() -> String { "64Gi".to_string() }

/// Node configuration
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct NodeConfig {
    /// Genesis hash to expect
    #[serde(default = "default_genesis_hash")]
    pub expected_genesis_hash: String,
    
    /// Limit ledger size in shreds
    #[serde(default = "default_ledger_size")]
    pub limit_ledger_size: i32,
    
    /// Enable full RPC API
    #[serde(default = "default_full_rpc")]
    pub full_rpc_api: bool,
    
    /// Enable accounts disk index
    #[serde(default = "default_accounts_index")]
    pub enable_accounts_disk_index: bool,
    
    /// Skip startup ledger verification
    #[serde(default = "default_skip_ledger_verify")]
    pub skip_startup_ledger_verification: bool,
    
    /// Number of RPC threads
    #[serde(default = "default_rpc_threads")]
    pub rpc_threads: i32,
    
    /// Maximum full snapshots to retain
    #[serde(default = "default_max_snapshots")]
    pub maximum_full_snapshots_to_retain: i32,
    
    /// WAL recovery mode
    #[serde(default = "default_wal_recovery")]
    pub wal_recovery_mode: String,
}

fn default_genesis_hash() -> String { "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d".to_string() }
fn default_ledger_size() -> i32 { 200000000 }
fn default_full_rpc() -> bool { true }
fn default_accounts_index() -> bool { true }
fn default_skip_ledger_verify() -> bool { true }
fn default_rpc_threads() -> i32 { 128 }
fn default_max_snapshots() -> i32 { 2 }
fn default_wal_recovery() -> String { "skip_any_corrupted_record".to_string() }

/// SolanaNodeStatus defines the observed state of SolanaNode
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct SolanaNodeStatus {
    /// Current phase
    pub phase: Option<NodePhase>,
    
    /// Current slot height
    pub slot_height: Option<i64>,
    
    /// Whether the node is healthy
    pub healthy: Option<bool>,
    
    /// Number of slots behind network
    pub slots_behind: Option<i32>,
}

/// Node phase
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodePhase {
    Pending,
    Initializing,
    Running,
    Error,
}
