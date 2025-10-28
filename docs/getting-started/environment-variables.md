# Environment Variables

Configure your Portal SDK Daemon with environment variables.

## Required Variables

### AUTH_TOKEN

**Description**: Authentication token for API access. This token must be provided by clients when connecting to the WebSocket API.

**Type**: String

**Security**: Generate a cryptographically secure random token. Never commit this to version control.

**Example**:
```bash
# Generate a secure token
openssl rand -hex 32

# Or use a password generator
pwgen -s 64 1
```

**Usage**:
```bash
AUTH_TOKEN=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

### NOSTR_KEY

**Description**: Your Portal instance's Nostr private key in hexadecimal format. This key is used to sign messages and authenticate your service on the Nostr network.

**Type**: Hexadecimal string (64 characters)

**Security**: Keep this key absolutely secret. Anyone with access to it can impersonate your Portal instance.

**Format**: Hex format (not nsec format)

**Converting from nsec**:
```bash
# If you have an nsec key, convert it to hex:
nak decode nsec1your-key-here
```

**Usage**:
```bash
NOSTR_KEY=5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a
```

## Optional Variables

### NWC_URL

**Description**: Nostr Wallet Connect URL for processing Lightning Network payments. This allows your Portal instance to request and receive payments on behalf of your service.

**Type**: String (nostr+walletconnect:// URL)

**Required for**: Payment processing (single and recurring payments)

**How to get**:
1. Use a Lightning wallet that supports NWC (Alby, Mutiny, etc.)
2. Navigate to wallet settings
3. Find "Nostr Wallet Connect" or "Wallet Connect String"
4. Copy the connection URL

**Example**:
```bash
NWC_URL=nostr+walletconnect://b889ff5b1513b641e2a139f661a661364979c5beee91842f8f0ef42ab558e9d4?relay=wss://relay.damus.io&secret=abcdef123456
```

**Without NWC**: Portal can still handle authentication and generate payment requests, but users will need to pay invoices manually.

### NOSTR_SUBKEY_PROOF

**Description**: Proof for Nostr subkey delegation. This is used when your Portal instance operates as a subkey delegated from a main key.

**Type**: String (delegation proof)

**Use case**: Advanced scenarios where you want to use a delegated subkey instead of a main key.

**Example**:
```bash
NOSTR_SUBKEY_PROOF=delegation-proof-string-here
```

### NOSTR_RELAYS

**Description**: Comma-separated list of Nostr relay URLs to connect to. Relays are used to publish and receive messages on the Nostr network.

**Type**: Comma-separated string

**Default**: If not specified, Portal uses a default set of popular public relays.

**Recommended relays**:
- `wss://relay.damus.io` - Popular, well-maintained
- `wss://relay.snort.social` - Fast and reliable
- `wss://nos.lol` - Good for payments
- `wss://relay.nostr.band` - Large relay network
- `wss://nostr.wine` - Paid relay (more reliable)

**Example**:
```bash
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol
```

**Considerations**:
- More relays = better redundancy but more bandwidth
- Include at least 3-5 relays for reliability
- Use relays that are geographically close to your users
- Consider using paid relays for production

## Configuration Examples

### Minimal Development Setup

Bare minimum for local development:

```bash
AUTH_TOKEN=dev-token-change-in-production
NOSTR_KEY=5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a
```

### Full Production Setup

Complete configuration for production deployment:

```bash
# Required
AUTH_TOKEN=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6
NOSTR_KEY=5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a

# Payment processing
NWC_URL=nostr+walletconnect://b889ff5b1513b641e2a139f661a661364979c5beee91842f8f0ef42ab558e9d4?relay=wss://relay.damus.io&secret=abcdef123456

# Network configuration
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol,wss://relay.nostr.band,wss://nostr.wine
```

### Using Environment Files

#### .env file (for docker-compose)

Create a `.env` file in your project directory:

```bash
# Portal Configuration
AUTH_TOKEN=your-secret-token
NOSTR_KEY=your-nostr-key-hex
NWC_URL=nostr+walletconnect://your-nwc-url
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.snort.social
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
  -e AUTH_TOKEN=$AUTH_TOKEN \
  -e NOSTR_KEY=$NOSTR_KEY \
  -e NWC_URL=$NWC_URL \
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
sed -i "s/AUTH_TOKEN=.*/AUTH_TOKEN=$NEW_TOKEN/" .env

# Restart Portal
docker-compose restart
```

## Validation

### Checking Current Configuration

```bash
# View environment variables in running container
docker exec portal-sdk-daemon env | grep -E 'AUTH_TOKEN|NOSTR_KEY|NWC_URL|NOSTR_RELAYS'

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

**Cause**: AUTH_TOKEN mismatch between server and client

**Solution**:
```bash
# Verify token in container
docker exec portal-sdk-daemon env | grep AUTH_TOKEN

# Check your SDK code uses the same token
```

### "Invalid NOSTR_KEY format"

**Cause**: Key is not in hex format or is invalid

**Solution**:
```bash
# Key should be 64 hex characters
echo $NOSTR_KEY | wc -c
# Should output: 65 (64 chars + newline)

# Verify it's valid hex
echo $NOSTR_KEY | grep -E '^[0-9a-f]{64}$'
```

### "Cannot connect to relays"

**Cause**: Invalid relay URLs or network issues

**Solution**:
```bash
# Test relay connectivity
wscat -c wss://relay.damus.io

# Verify relay URLs are correct (must start with wss://)
echo $NOSTR_RELAYS | tr ',' '\n'
```

---

**Next Steps**:
- [Docker Deployment](docker-deployment.md)
- [Building from Source](building-from-source.md)
- [Quick Start Guide](quick-start.md)

