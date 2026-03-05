# Production Dockerfile for CTO Blockchain Operator
FROM rust:1.86-bookworm AS builder

WORKDIR /app

# Build deps cache
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 cto

COPY --from=builder /app/target/release/cto-blockchain-operator /usr/local/bin/cto-blockchain-operator

USER 10001

ENTRYPOINT ["/usr/local/bin/cto-blockchain-operator"]
