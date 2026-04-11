# Building from Source

**Prerequisites:** Rust 1.70+ (rustup), Git. Optional: Nix for reproducible builds.

## Clone and build

```bash
git clone https://github.com/PortalTechnologiesInc/lib.git
cd lib
```

**REST API (portal-rest):**

```bash
cargo build --package portal-rest --release
./target/release/rest
```

**Nix:**

```bash
nix build .#rest
./result/bin/rest
```

**Docker image:** nix build .#rest-docker then docker load &lt; result.

## Other targets

- **CLI:** cargo build --package portal-cli --release → ./target/release/portal-cli
- **TS client:** cd crates/portal-rest/clients/ts && npm install && npm run build

## Run with env

```bash
PORTAL__AUTH__AUTH_TOKEN=dev-token \
PORTAL__NOSTR__PRIVATE_KEY=your-key-hex \
cargo run --package portal-rest --release
```

Config can come from ~/.portal-rest/config.toml or env; see [Environment variables](environment-variables.md).

## Cross-build (Nix)

```bash
nix build .#rest --system x86_64-linux
nix build .#rest --system aarch64-darwin
```

## Troubleshooting

- **Missing OpenSSL:** Install libssl-dev (Debian/Ubuntu) or openssl (macOS); set PKG_CONFIG_PATH if needed.
- **Linker:** Linux: sudo apt-get install build-essential; macOS: xcode-select --install.
- **Nix:** nix flake update and nix build -L .#rest for verbose output.

---

- [Environment variables](environment-variables.md) · [Docker](docker-deployment.md) · [Contributing](../resources/contributing.md)
