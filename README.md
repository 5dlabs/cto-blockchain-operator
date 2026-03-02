# CTO Blockchain Operator

A cloud-agnostic blockchain infrastructure operator built from scratch in Rust for deploying and managing blockchain nodes on bare-metal and cloud providers with a focus on performance, interoperability, and AI agent infrastructure.

## Overview

The CTO Blockchain Operator extends Kubernetes with custom resources for deploying blockchain infrastructure across multiple providers including:
- Bare-metal providers (Cherry Servers, Latitude.sh, OVH)
- Cloud providers (AWS, GCP, Azure - planned)

Unlike existing solutions, this operator is built from scratch in Rust with performance and extensibility as core principles.

## Supported Blockchains

### L1 Blockchains
| Chain | Node Types | Hardware Requirements |
|-------|------------|----------------------|
| **Solana** | Validator, RPC, Archival | 32 cores, 256GB RAM, 4x NVMe SSD |
| **Sui** | Validator | 24 cores, 128GB RAM, 4TB NVMe |
| **Aptos** | Validator, Fullnode | 32 cores, 64GB RAM, 3TB SSD |
| **Monad** | Validator | 16 cores (4.5GHz+), 32GB RAM, 2.5TB SSD |
| **NEAR** | Validator, RPC Node | 8 cores, 48GB RAM, 3TB NVMe |
| **Berachain** | Validator, RPC Node | 4 cores, 16GB RAM, 1TB SSD |

### L2 Blockchains
| Chain | Node Types | Hardware Requirements |
|-------|------------|----------------------|
| **Arbitrum** | Full Node | 8+ cores, 64GB RAM, NVMe SSD |
| **Base** | Full Node | 8+ cores, 64GB RAM, NVMe SSD |
| **Optimism** | Full Node | 8+ cores, 64GB RAM, NVMe SSD |

### Interoperability Protocols
| Protocol | Components | Hardware Requirements |
|----------|------------|----------------------|
| **LayerZero** | Relayer, Oracle | 8 cores, 32GB RAM |

## Architecture

Built with modern Rust and Kubernetes controller-runtime:

```
┌────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                      │
├────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌──────────────────────────────┐  │
│  │                 │    │                              │  │
│  │  CTO Operator   │◄──►│      Custom Resources        │  │
│  │    (Rust)       │    │                              │  │
│  └─────────────────┘    └──────────────────────────────┘  │
│           │                                               │
│           ▼                                               │
│  ┌─────────────────┐                                      │
│  │                 │                                      │
│  │  Controllers    │                                      │
│  │                 │                                      │
│  └─────────────────┘                                      │
│           │                                               │
│           ▼                                               │
│  ┌─────────────────┐    ┌──────────────────────────────┐  │
│  │                 │    │                              │  │
│  │  Providers      │◄──►│   Infrastructure            │  │
│  │                 │    │                              │  │
│  └─────────────────┘    └──────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites
- Kubernetes cluster (v1.24+)
- cert-manager installed
- Access to provider credentials

### Installation

```bash
# Apply CRDs
kubectl apply -f config/crd/bases/

# Deploy the operator
kubectl apply -f deploy/

# Create a Solana node
kubectl apply -f examples/solana-validator.yaml
```

### Testing with KinD

```bash
# Create test cluster
kind create cluster

# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.14.5/cert-manager.yaml
kubectl wait --for=condition=available --timeout=300s deployment/cert-manager -n cert-manager

# Apply CRDs and deploy example
kubectl apply -f config/crd/bases/
kubectl apply -f examples/solana-validator.yaml
```

## Providers

### Cherry Servers
```rust
let provider = CherryProvider::new(
    api_key,
    team_id,
    project_id,
);
```

### Latitude
```rust
let provider = LatitudeProvider::new(api_key);
```

### OVH
```rust
let provider = OvhProvider::new(
    endpoint,
    app_key,
    app_secret,
    consumer_key,
);
```

## Custom Resources

### SolanaNode

```yaml
apiVersion: blockchain.5dlabs.io/v1alpha1
kind: SolanaNode
metadata:
  name: mainnet-validator
spec:
  nodeType: validator
  enableVoting: false
  identitySecret: "solana-identity"
  knownValidators:
    - "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2"
  entrypoints:
    - "entrypoint.mainnet-beta.solana.com:8001"
  config:
    expectedGenesisHash: "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
    fullRpcApi: true
    rpcThreads: 128
```

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests with KinD (GitHub Actions)
# See .github/workflows/test-kind.yaml
```

## License

Apache 2.0
