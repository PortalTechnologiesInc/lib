# Building from Source

Build Portal from source code for development or custom deployments.

## Prerequisites

### Required Tools

1. **Rust Toolchain** (1.70+)
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

2. **Git**
```bash
# Verify git is installed
git --version
```

### Optional Tools

For building with Nix (recommended for reproducible builds):

```bash
# Install Nix
curl -L https://nixos.org/nix/install | sh

# Enable flakes (if not already enabled)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

## Clone the Repository

```bash
git clone https://github.com/PortalTechnologiesInc/lib.git
cd portal
```

## Building with Cargo

### Build the REST API

```bash
# Build in debug mode (faster compilation, slower runtime)
cargo build --package portal-rest

# Build in release mode (optimized)
cargo build --package portal-rest --release

# Run the binary
./target/release/rest
```

### Build All Components

```bash
# Build everything
cargo build --release

# Build specific components
cargo build --package app --release
cargo build --package portal-cli --release
cargo build --package portal-rates --release
```

### Build the JavaScript/TypeScript client

```bash
cd crates/portal-rest/clients/ts

# Install dependencies
npm install

# Build the client
npm run build

# Run tests
npm test

# Build for production
npm run build:production
```

## Building with Nix

Nix provides reproducible, deterministic builds:

### Build the REST API

```bash
# Build the REST API server
nix build .#rest

# Run it
./result/bin/rest
```

### Build Docker Image

```bash
# Build Docker image for your architecture
nix build .#rest-docker

# Load into Docker
docker load < result

# Tag and run
docker tag portal-rest:latest portal:local
docker run -p 3000:3000 portal:local
```

### Build for Different Architectures

```bash
# Build for x86_64 Linux
nix build .#rest --system x86_64-linux

# Build for ARM64 Linux
nix build .#rest --system aarch64-linux

# Build for macOS
nix build .#rest --system aarch64-darwin
```

## Development Setup

### Set Up Development Environment

```bash
# Enter Nix development shell (if using Nix)
nix develop

# Or set up manually with Cargo
cargo install cargo-watch
cargo install cargo-edit
```

### Run in Development Mode

```bash
# Run REST API with auto-reload
cargo watch -x 'run --package portal-rest'

# Run with environment variables
PORTAL__AUTH__AUTH_TOKEN=dev-token \
PORTAL__NOSTR__PRIVATE_KEY=your-key-hex \
cargo run --package portal-rest

# Or use the env.example (if it exists)
# cp crates/portal-rest/env.example crates/portal-rest/.env
# Edit .env with your values
# source crates/portal-rest/.env
cargo run --package portal-rest
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test --package portal-rest
cargo test --package app

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Building the JavaScript/TypeScript client

### Development Build

```bash
cd crates/portal-rest/clients/ts

# Install dependencies
npm install

# Build in watch mode
npm run build -- --watch

# Run example
npm run example
```

### Production Build

```bash
# Build optimized version
npm run build

# Create package
npm pack

# Publish to npm (requires authentication)
npm publish
```

## Building the CLI

```bash
# Build the CLI tool
cargo build --package portal-cli --release

# Run it
./target/release/portal-cli --help

# Install globally
cargo install --path crates/portal-cli
```

## Cross-Compilation

### Linux → Windows

```bash
# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Install mingw-w64
# On Ubuntu/Debian:
sudo apt-get install mingw-w64

# Build
cargo build --package portal-rest --target x86_64-pc-windows-gnu --release
```

### Linux → macOS

Cross-compiling to macOS requires osxcross. Using Nix is easier:

```bash
nix build .#rest --system x86_64-darwin
nix build .#rest --system aarch64-darwin
```

## Optimizations

### Size Optimization

Edit `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Enable Link Time Optimization
codegen-units = 1    # Better optimization
strip = true         # Strip symbols
```

Build:
```bash
cargo build --package portal-rest --release
```

### Performance Optimization

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "fat"          # Full LTO
codegen-units = 1
```

### Development Optimization

```toml
[profile.dev]
opt-level = 1        # Some optimization for faster dev builds
```

## Platform-Specific Builds

### Linux

```bash
# Build with system libraries
cargo build --package portal-rest --release

# Build static binary (Linux only)
cargo build --package portal-rest --release --target x86_64-unknown-linux-musl
```

### macOS

```bash
# Build for current architecture
cargo build --package portal-rest --release

# Build universal binary (both Intel and Apple Silicon)
cargo build --package portal-rest --release --target x86_64-apple-darwin
cargo build --package portal-rest --release --target aarch64-apple-darwin

# Create universal binary
lipo -create \
  target/x86_64-apple-darwin/release/rest \
  target/aarch64-apple-darwin/release/rest \
  -output target/release/rest-universal
```

### Windows

```bash
# Build for Windows
cargo build --package portal-rest --release --target x86_64-pc-windows-msvc
```

## Creating Releases

### Binary Releases

```bash
# Build all release binaries
cargo build --release --workspace

# Create release directory
mkdir -p releases/portal-v1.0.0

# Copy binaries
cp target/release/rest releases/portal-v1.0.0/
cp target/release/portal-cli releases/portal-v1.0.0/

# Create tarball
tar -czf portal-v1.0.0-linux-x86_64.tar.gz -C releases portal-v1.0.0/
```

### Docker Release

```bash
# Build Docker image
nix build .#rest-docker
docker load < result

# Tag for release
docker tag portal-rest:latest getportal/sdk-daemon:v1.0.0
docker tag portal-rest:latest getportal/sdk-daemon:latest

# Push to registry
docker push getportal/sdk-daemon:v1.0.0
docker push getportal/sdk-daemon:latest
```

## Troubleshooting

### Compilation Errors

**"cannot find -lssl"**
```bash
# Install OpenSSL development libraries
# Ubuntu/Debian:
sudo apt-get install libssl-dev pkg-config

# macOS:
brew install openssl pkg-config

# Set PKG_CONFIG_PATH if needed
export PKG_CONFIG_PATH=/usr/local/opt/openssl/lib/pkgconfig
```

**"linker 'cc' not found"**
```bash
# Install build essentials
# Ubuntu/Debian:
sudo apt-get install build-essential

# macOS:
xcode-select --install
```

### Slow Compilation

```bash
# Use faster linker (Linux)
sudo apt-get install lld
echo '[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]' >> ~/.cargo/config.toml

# Or use mold (even faster)
cargo install mold
```

### Out of Memory

```bash
# Limit parallel jobs
cargo build --jobs 2 --release

# Or set in config
echo '[build]
jobs = 2' >> ~/.cargo/config.toml
```

### Nix Build Issues

```bash
# Clean build cache
nix-collect-garbage

# Update flake inputs
nix flake update

# Build with verbose output
nix build -L .#rest
```

## Development Workflow

### Recommended Workflow

1. **Make changes** to source code
2. **Run tests**: `cargo test`
3. **Check code**: `cargo clippy`
4. **Format code**: `cargo fmt`
5. **Build**: `cargo build --release`
6. **Test locally**: Run the binary
7. **Commit changes**: `git commit`

### Pre-commit Hooks

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "Running cargo fmt..."
cargo fmt --all -- --check

echo "Running cargo clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "Running tests..."
cargo test --all

echo "All checks passed!"
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

---

**Next Steps**:
- [Environment Variables](environment-variables.md)
- [Docker Deployment](docker-deployment.md)
- [Contributing Guide](../resources/contributing.md)

