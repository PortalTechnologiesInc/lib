# Environment Variables

Configure your Portal SDK Daemon (portal-rest) with environment variables. The binary reads config from `~/.portal-rest/config.toml` and overrides via `PORTAL__<SECTION>__<KEY>` env vars.

## Config file and PORTAL__* env vars

- **Config file:** `~/.portal-rest/config.toml` (created with defaults if missing). Copy from `example.config.toml` in the `portal-rest` crate.
- **Env overrides:** Any setting can be overridden with `PORTAL__<SECTION>__<KEY>=value` (double underscores). Section and key match the TOML structure.

| Config key | Env variable | Description |
|------------|--------------|-------------|
| `info.listen_port` | `PORTAL__INFO__LISTEN_PORT` | Port the API listens on (default 3000). |
| `auth.auth_token` | `PORTAL__AUTH__AUTH_TOKEN` | API auth token. Required for clients to connect. |
| `nostr.private_key` | `PORTAL__NOSTR__PRIVATE_KEY` | Nostr private key in hex format. Required. |
| `nostr.relays` | `PORTAL__NOSTR__RELAYS` | Comma-separated relay URLs. |
| `nostr.subkey_proof` | `PORTAL__NOSTR__SUBKEY_PROOF` | Proof for Nostr subkey delegation (optional). |
| `wallet.ln_backend` | `PORTAL__WALLET__LN_BACKEND` | `none`, `nwc`, or `breez`. |
| `wallet.nwc.url` | `PORTAL__WALLET__NWC__URL` | Nostr Wallet Connect URL (when `ln_backend=nwc`). |
| `wallet.breez.api_key` | `PORTAL__WALLET__BREEZ__API_KEY` | Breez API key (when `ln_backend=breez`). |
| `wallet.breez.storage_dir` | `PORTAL__WALLET__BREEZ__STORAGE_DIR` | Breez storage directory. |
| `wallet.breez.mnemonic` | `PORTAL__WALLET__BREEZ__MNEMONIC` | Breez mnemonic (when `ln_backend=breez`). |

**Run from config:**

```bash
portal-rest   # uses ~/.portal-rest/config.toml
```

## Required variables

### PORTAL__AUTH__AUTH_TOKEN

**Description**: Authentication token for API access. Clients must provide this token when connecting to the WebSocket API.

**Security**: Generate a cryptographically secure random token. Never commit this to version control.

```bash
# Generate a secure token
openssl rand -hex 32
```

### PORTAL__NOSTR__PRIVATE_KEY

**Description**: Your Portal instance's Nostr private key in hexadecimal format. Used to sign messages and authenticate on the Nostr network.

**Format**: Hex format (64 characters). Convert from nsec with: `nak decode nsec1your-key-here`

## Optional variables

### PORTAL__WALLET__NWC__URL (for payments)

**Description**: Nostr Wallet Connect URL for processing Lightning Network payments. Set `PORTAL__WALLET__LN_BACKEND=nwc` when using this.

**Without NWC**: Portal can still handle authentication and generate payment requests, but users will need to pay invoices manually.

### PORTAL__NOSTR__RELAYS

**Description**: Comma-separated list of Nostr relay URLs. Default comes from config file.

**Recommended relays:** `wss://relay.damus.io`, `wss://relay.snort.social`, `wss://nos.lol`, `wss://relay.nostr.band`

## Configuration Examples

### Minimal Development Setup

Bare minimum for local development:

```bash
PORTAL__AUTH__AUTH_TOKEN=dev-token-change-in-production \
PORTAL__NOSTR__PRIVATE_KEY=5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a \
portal-rest
```

### Full Production Setup

Complete configuration for production deployment:

```bash
# Required
export PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32)
export PORTAL__NOSTR__PRIVATE_KEY=5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a

# Payment processing
export PORTAL__WALLET__LN_BACKEND=nwc
export PORTAL__WALLET__NWC__URL=nostr+walletconnect://...

# Network configuration
export PORTAL__NOSTR__RELAYS=wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol,wss://relay.nostr.band

portal-rest
```

### Using Environment Files

#### .env file (for docker-compose)

Create a `.env` file in your project directory:

```bash
# Portal Configuration (use PORTAL__* format)
PORTAL__AUTH__AUTH_TOKEN=your-secret-token
PORTAL__NOSTR__PRIVATE_KEY=your-nostr-key-hex
PORTAL__WALLET__LN_BACKEND=nwc
PORTAL__WALLET__NWC__URL=nostr+walletconnect://your-nwc-url
PORTAL__NOSTR__RELAYS=wss://relay.damus.io,wss://relay.snort.social
```

**Important**: Add `.env` to your `.gitignore`:
```bash
echo ".env" >> .gitignore
```

#### Using with Docker

```bash
# Load from .env file
docker run --env-file .env -p 3000:3000 getportal/sdk-daemon:latest

# Or pass variables directly
docker run \
  -e PORTAL__AUTH__AUTH_TOKEN=$PORTAL__AUTH__AUTH_TOKEN \
  -e PORTAL__NOSTR__PRIVATE_KEY=$PORTAL__NOSTR__PRIVATE_KEY \
  -e PORTAL__WALLET__LN_BACKEND=nwc \
  -e PORTAL__WALLET__NWC__URL=$PORTAL__WALLET__NWC__URL \
  -p 3000:3000 \
  getportal/sdk-daemon:latest
```

#### Using with Docker Compose

```yaml
version: '3.8'

services:
  portal:
    image: getportal/sdk-daemon:latest
    env_file:
      - .env
    ports:
      - "3000:3000"
```

## Security Best Practices

### 1. Generate Strong Tokens

```bash
# Use openssl
openssl rand -base64 32

# Or use a dedicated tool
pwgen -s 64 1

# On Linux/macOS
head -c 32 /dev/urandom | base64
```

### 2. Secure Storage

**DO**:
- Store secrets in environment variables
- Use secret management systems (AWS Secrets Manager, HashiCorp Vault)
- Encrypt secrets at rest
- Rotate tokens regularly

**DON'T**:
- Commit secrets to version control
- Include secrets in Docker images
- Share secrets in plain text
- Hardcode secrets in application code

### 3. Access Control

```bash
# Set proper file permissions for .env files
chmod 600 .env

# Verify permissions
ls -l .env
# Should show: -rw------- (only owner can read/write)
```

### 4. Secret Rotation

Regularly rotate your secrets:

```bash
# Generate new AUTH_TOKEN
NEW_TOKEN=$(openssl rand -hex 32)

# Update in .env
sed -i "s/PORTAL__AUTH__AUTH_TOKEN=.*/PORTAL__AUTH__AUTH_TOKEN=$NEW_TOKEN/" .env

# Restart Portal
docker-compose restart
```

## Validation

### Checking Current Configuration

```bash
# View environment variables in running container
docker exec portal-sdk-daemon env | grep PORTAL__

# Note: This will show your secrets! Only use for debugging
```

### Testing Configuration

```bash
# Test health endpoint
curl http://localhost:3000/health

# Test WebSocket connection
wscat -c ws://localhost:3000/ws

# Send auth command
{"id":"test","cmd":"Auth","params":{"token":"your-auth-token"}}
```

## Troubleshooting

### "Authentication failed"

**Cause**: Auth token mismatch between server and client

**Solution**:
```bash
# Verify token in container
docker exec portal-sdk-daemon env | grep PORTAL__AUTH__AUTH_TOKEN

# Check your SDK code uses the same token
```

### "Invalid Nostr key format"

**Cause**: Key is not in hex format or is invalid

**Solution**:
```bash
# Key should be 64 hex characters
echo $PORTAL__NOSTR__PRIVATE_KEY | wc -c
# Should output: 65 (64 chars + newline)

# Verify it's valid hex
echo $PORTAL__NOSTR__PRIVATE_KEY | grep -E '^[0-9a-f]{64}$'
```

### "Cannot connect to relays"

**Cause**: Invalid relay URLs or network issues

**Solution**:
```bash
# Test relay connectivity
wscat -c wss://relay.damus.io

# Verify relay URLs are correct (must start with wss://)
echo $PORTAL__NOSTR__RELAYS | tr ',' '\n'
```

---

**Next Steps**:
- [Docker Deployment](docker-deployment.md)
- [Building from Source](building-from-source.md)
- [Quick Start Guide](quick-start.md)

