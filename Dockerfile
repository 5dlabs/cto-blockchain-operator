# Simple placeholder Dockerfile for CTO Blockchain Operator
# This will allow us to deploy and test the operator framework
# The actual Rust implementation can be added once the build is fixed

FROM debian:bookworm-slim

# Install required packages
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 cto

# Create a simple placeholder binary
RUN echo '#!/bin/bash\nwhile true; do sleep 3600; done' > /usr/local/bin/cto-blockchain-operator && \
    chmod +x /usr/local/bin/cto-blockchain-operator

# Expose metrics port
EXPOSE 8080

# Run the application
ENTRYPOINT ["/usr/local/bin/cto-blockchain-operator"]
