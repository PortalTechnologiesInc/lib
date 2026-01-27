FROM rust:1.88-slim as builder

# Build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/portal-rest/Cargo.toml ./crates/portal-rest/
COPY crates/portal-app/Cargo.toml ./crates/portal-app/
COPY crates/portal-cli/Cargo.toml ./crates/portal-cli/
COPY crates/portal-sdk/Cargo.toml ./crates/portal-sdk/
COPY crates/portal-rates/Cargo.toml ./crates/portal-rates/
COPY crates/portal-wallet/Cargo.toml ./crates/portal-wallet/
COPY crates/portal-fetch-git-hash/Cargo.toml ./crates/portal-fetch-git-hash/
COPY crates/portal/Cargo.toml ./crates/portal/

COPY crates/portal-rest/src ./crates/portal-rest/src
COPY crates/portal-rest/example.config.toml ./crates/portal-rest/
COPY crates/portal-app/src ./crates/portal-app/src
COPY crates/portal-cli/src ./crates/portal-cli/src
COPY crates/portal-sdk/src ./crates/portal-sdk/src
COPY crates/portal-rates/src ./crates/portal-rates/src
COPY crates/portal-rates/assets ./crates/portal-rates/assets
COPY crates/portal-wallet/src ./crates/portal-wallet/src
COPY crates/portal-fetch-git-hash/src ./crates/portal-fetch-git-hash/src
COPY crates/portal/src ./crates/portal/src

RUN cargo build --release -p portal-rest

FROM debian:bookworm-slim

# Runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 portal

WORKDIR /app

COPY --from=builder /app/target/release/rest /app/rest
RUN chown -R portal:portal /app

USER portal

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/app/rest"]