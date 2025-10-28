# Docker Deployment

Deploy the Portal SDK Daemon using Docker for easy setup and management.

## Quick Deployment

### Using Pre-built Image

The easiest way to run Portal is using the official Docker image:

```bash
docker run --rm --name portal-sdk-daemon -d \
  -p 3000:3000 \
  -e AUTH_TOKEN=your-secret-token \
  -e NOSTR_KEY=your-nostr-private-key \
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
      - AUTH_TOKEN=${AUTH_TOKEN}
      - NOSTR_KEY=${NOSTR_KEY}
      - NWC_URL=${NWC_URL:-}
      - NOSTR_RELAYS=${NOSTR_RELAYS:-}
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
AUTH_TOKEN=your-secret-token-here
NOSTR_KEY=your-nostr-private-key-hex
NWC_URL=nostr+walletconnect://...
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.snort.social
```

Start the service:

```bash
docker-compose up -d
```

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AUTH_TOKEN` | Secret token for API authentication | `random-secret-token-12345` |
| `NOSTR_KEY` | Nostr private key in hex format | `5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab7a` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NWC_URL` | Nostr Wallet Connect URL for processing payments | None |
| `NOSTR_SUBKEY_PROOF` | Proof for Nostr subkey delegation | None |
| `NOSTR_RELAYS` | Comma-separated list of relay URLs | Common public relays |

## Configuration Examples

### Development Setup

For local development with minimal configuration:

```bash
docker run --rm --name portal-dev \
  -p 3000:3000 \
  -e AUTH_TOKEN=dev-token \
  -e NOSTR_KEY=$(cat ~/.nostr/key.hex) \
  getportal/sdk-daemon:latest
```

### Production Setup

For production with all features enabled:

```bash
docker run -d \
  --name portal-production \
  --restart unless-stopped \
  -p 3000:3000 \
  -e AUTH_TOKEN=$(openssl rand -hex 32) \
  -e NOSTR_KEY=$(cat /secure/nostr-key.hex) \
  -e NWC_URL="nostr+walletconnect://..." \
  -e NOSTR_RELAYS="wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol" \
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
  -e AUTH_TOKEN=your-token \
  -e NOSTR_KEY=your-key \
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
  -e AUTH_TOKEN=your-token \
  -e NOSTR_KEY=your-key \
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

### Building from Source with Nix

Portal uses Nix for reproducible builds:

```bash
# Clone the repository
git clone https://github.com/PortalTechnologiesInc/lib.git
cd portal

# Build the Docker image
nix build .#rest-docker

# Load into Docker
docker load < result
```

### Multi-Architecture Builds

To build for multiple architectures:

```bash
# On amd64 machine
nix build .#rest-docker
docker tag portal-rest:latest getportal/sdk-daemon:amd64
docker push getportal/sdk-daemon:amd64

# On arm64 machine
nix build .#rest-docker
docker tag portal-rest:latest getportal/sdk-daemon:arm64
docker push getportal/sdk-daemon:arm64

# Create and push manifest
docker manifest create getportal/sdk-daemon:latest \
  --amend getportal/sdk-daemon:amd64 \
  --amend getportal/sdk-daemon:arm64
docker manifest push getportal/sdk-daemon:latest
```

## Container Management

### Viewing Logs

```bash
# Follow logs in real-time
docker logs -f portal-sdk-daemon

# View last 100 lines
docker logs --tail 100 portal-sdk-daemon

# View logs with timestamps
docker logs -t portal-sdk-daemon
```

### Monitoring Health

```bash
# Check container status
docker ps -f name=portal-sdk-daemon

# Check health status
docker inspect --format='{{.State.Health.Status}}' portal-sdk-daemon

# Test health endpoint directly
curl http://localhost:3000/health
```

### Restarting the Service

```bash
# Restart container
docker restart portal-sdk-daemon

# Stop container
docker stop portal-sdk-daemon

# Remove container
docker rm portal-sdk-daemon
```

### Updating to Latest Version

```bash
# Pull latest image
docker pull getportal/sdk-daemon:latest

# Stop and remove old container
docker stop portal-sdk-daemon
docker rm portal-sdk-daemon

# Start new container
docker run -d \
  --name portal-sdk-daemon \
  -p 3000:3000 \
  -e AUTH_TOKEN=your-token \
  -e NOSTR_KEY=your-key \
  getportal/sdk-daemon:latest
```

## Security Considerations

1. **Never commit secrets**: Don't include `AUTH_TOKEN` or `NOSTR_KEY` in version control
2. **Use strong tokens**: Generate cryptographically secure random tokens
3. **Restrict network access**: Use firewalls to limit who can connect
4. **Enable HTTPS**: Use a reverse proxy with SSL/TLS
5. **Regular updates**: Keep the Docker image updated
6. **Monitor logs**: Watch for suspicious activity

## Troubleshooting

### Container won't start

```bash
# Check logs for errors
docker logs portal-sdk-daemon

# Verify environment variables
docker inspect portal-sdk-daemon | grep -A 20 Env
```

### Health check failing

```bash
# Test health endpoint
curl http://localhost:3000/health

# Check if port is accessible
netstat -tlnp | grep 3000

# Verify container is running
docker ps -a
```

### Permission issues

```bash
# Run with specific user
docker run -d \
  --user 1000:1000 \
  --name portal \
  -p 3000:3000 \
  -e AUTH_TOKEN=token \
  -e NOSTR_KEY=key \
  getportal/sdk-daemon:latest
```

---

**Next Steps**: 
- [Configure environment variables](environment-variables.md)
- [Install the TypeScript SDK](../sdk/installation.md)
- [Build from source](building-from-source.md)

