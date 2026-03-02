FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY config/crd/bases/ config/crd/bases/

# Copy source code
COPY . .

# Build the application
RUN cargo build --release

# Production image
FROM debian:bookworm-slim

# Install ca-certificates for TLS
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 cto

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/cto-blockchain-operator /usr/local/bin/cto-blockchain-operator

# Change ownership and switch to non-root user
RUN chown cto:cto /usr/local/bin/cto-blockchain-operator
USER cto

# Expose metrics port
EXPOSE 8080

# Run the application
ENTRYPOINT ["/usr/local/bin/cto-blockchain-operator"]
