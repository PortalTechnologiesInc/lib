# Docker Deployment

Deploy the Portal SDK Daemon using Docker for easy setup and management.

## Quick Deployment

### Using Pre-built Image

The easiest way to run Portal is using the official Docker image:

```bash
docker run --rm --name portal-sdk-daemon -d \
  -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

### With Docker Compose

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  portal:
    image: getportal/sdk-daemon:latest
    container_name: portal-sdk-daemon
    ports:
      - "3000:3000"
    environment:
      - PORTAL__AUTH__AUTH_TOKEN=${PORTAL__AUTH__AUTH_TOKEN}
      - PORTAL__NOSTR__PRIVATE_KEY=${PORTAL__NOSTR__PRIVATE_KEY}
      - PORTAL__WALLET__LN_BACKEND=${PORTAL__WALLET__LN_BACKEND:-none}
      - PORTAL__WALLET__NWC__URL=${PORTAL__WALLET__NWC__URL:-}
      - PORTAL__NOSTR__RELAYS=${PORTAL__NOSTR__RELAYS:-}
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

Create a `.env` file:

```bash
PORTAL__AUTH__AUTH_TOKEN=your-secret-token-here
PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex
PORTAL__WALLET__LN_BACKEND=nwc
PORTAL__WALLET__NWC__URL=nostr+walletconnect://...
PORTAL__NOSTR__RELAYS=wss://relay.damus.io,wss://relay.snort.social
```

Start the service:

```bash
docker-compose up -d
```

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `PORTAL__AUTH__AUTH_TOKEN` | Secret token for API authentication | `random-secret-token-12345` |
| `PORTAL__NOSTR__PRIVATE_KEY` | Nostr private key in hex format | `5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORTAL__WALLET__LN_BACKEND` | Wallet backend: `none`, `nwc`, or `breez` | `none` |
| `PORTAL__WALLET__NWC__URL` | Nostr Wallet Connect URL (when `ln_backend=nwc`) | None |
| `PORTAL__NOSTR__SUBKEY_PROOF` | Proof for Nostr subkey delegation | None |
| `PORTAL__NOSTR__RELAYS` | Comma-separated list of relay URLs | From config |

## Configuration Examples

### Development Setup

For local development with minimal configuration:

```bash
docker run --rm --name portal-dev \
  -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=dev-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=$(cat ~/.nostr/key.hex) \
  getportal/sdk-daemon:latest
```

### Production Setup

For production with all features enabled:

```bash
docker run -d \
  --name portal-production \
  --restart unless-stopped \
  -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32) \
  -e PORTAL__NOSTR__PRIVATE_KEY=$(cat /secure/nostr-key.hex) \
  -e PORTAL__WALLET__LN_BACKEND=nwc \
  -e PORTAL__WALLET__NWC__URL="nostr+walletconnect://..." \
  -e PORTAL__NOSTR__RELAYS="wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol" \
  --health-cmd="curl -f http://localhost:3000/health || exit 1" \
  --health-interval=30s \
  --health-timeout=10s \
  --health-retries=3 \
  getportal/sdk-daemon:latest
```

### With Persistent Storage

If you need to persist data (like session information):

```bash
docker run -d \
  --name portal \
  -p 3000:3000 \
  -v portal-data:/app/data \
  -e PORTAL__AUTH__AUTH_TOKEN=your-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-key \
  getportal/sdk-daemon:latest
```

## Network Configuration

### Exposing to External Networks

By default, Portal listens on all interfaces (0.0.0.0:3000). To expose it externally:

```bash
# Bind to specific host interface
docker run -d \
  --name portal \
  -p 192.168.1.100:3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-key \
  getportal/sdk-daemon:latest
```

### Behind a Reverse Proxy

For production, use a reverse proxy like Nginx or Caddy:

**Nginx configuration:**
```nginx
server {
    listen 443 ssl http2;
    server_name portal.yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Caddy configuration:**
```
portal.yourdomain.com {
    reverse_proxy localhost:3000
}
```

## Building Custom Images

### Building from Dockerfile

Build the image from the repository's `Dockerfile`:

```bash
# Clone the repository
git clone https://github.com/PortalTechnologiesInc/lib.git
cd lib

# Build the image
docker build -t portal-rest:latest .

# Run it
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  portal-rest:latest
```

**With Docker Compose**, create `docker-compose.yml`:

```yaml
services:
  portal:
    build: .
    image: portal-rest:latest
    container_name: portal-sdk-daemon
    ports:
      - "3000:3000"
    environment:
      - PORTAL__AUTH__AUTH_TOKEN=${PORTAL__AUTH__AUTH_TOKEN}
      - PORTAL__NOSTR__PRIVATE_KEY=${PORTAL__NOSTR__PRIVATE_KEY}
      - PORTAL__WALLET__LN_BACKEND=${PORTAL__WALLET__LN_BACKEND:-none}
      - PORTAL__WALLET__NWC__URL=${PORTAL__WALLET__NWC__URL:-}
      - PORTAL__NOSTR__RELAYS=${PORTAL__NOSTR__RELAYS:-}
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

Create a `.env` file with your values, then:

```bash
docker compose up -d
```

### Building from Source with Nix

Portal uses Nix for reproducible builds:

```bash
# Clone the repository
git clone https://github.com/PortalTechnologiesInc/lib.git
cd lib

# Build the Docker image
nix build .#rest-docker

# Load into Docker
docker load < result
```

## Security Considerations

1. **Never commit secrets**: Don't include `PORTAL__AUTH__AUTH_TOKEN` or `PORTAL__NOSTR__PRIVATE_KEY` in version control
2. **Use strong tokens**: Generate cryptographically secure random tokens
3. **Restrict network access**: Use firewalls to limit who can connect
4. **Enable HTTPS**: Use a reverse proxy with SSL/TLS
5. **Regular updates**: Keep the Docker image updated
6. **Monitor logs**: Watch for suspicious activity

---

**Next Steps**: 
- [Configure environment variables](environment-variables.md)
- [Install the SDK](../sdk/installation.md)
- [Build from source](building-from-source.md)

