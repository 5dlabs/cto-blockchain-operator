FROM rust:1.87-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 cto

COPY --from=builder /app/target/release/cto-blockchain-operator /usr/local/bin/cto-blockchain-operator

USER cto
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/cto-blockchain-operator"]
