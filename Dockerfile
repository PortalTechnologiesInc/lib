FROM rust:1.88-slim as builder

# Build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY rest/Cargo.toml ./rest/
COPY app/Cargo.toml ./app/
COPY cli/Cargo.toml ./cli/
COPY sdk/Cargo.toml ./sdk/
COPY rates/Cargo.toml ./rates/
COPY wallet/Cargo.toml ./wallet/
COPY fetch-git-hash/Cargo.toml ./fetch-git-hash/

COPY rest/src ./rest/src
COPY rest/example.config.toml ./rest/
COPY app/src ./app/src
COPY cli/src ./cli/src
COPY sdk/src ./sdk/src
COPY rates/src ./rates/src
COPY wallet/src ./wallet/src
COPY fetch-git-hash/src ./fetch-git-hash/src
COPY src ./src
COPY assets ./assets

RUN cargo build --release -p rest

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