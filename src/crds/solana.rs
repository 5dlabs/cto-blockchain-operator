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
    /// Provider for bare-metal server
    #[serde(default = "default_provider")]
    pub provider: Provider,
    
    /// Region for the bare-metal server (e.g., "lt-siauliai", "nl-ams")
    #[serde(default = "default_region")]
    pub region: String,
    
    /// Number of replicas
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
fn default_region() -> String { "lt-siauliai".to_string() }
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
