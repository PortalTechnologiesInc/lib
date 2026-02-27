# Docker Deployment

## Run with pre-built image

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:0.3.0
```

> **Tip:** pin a specific version in production (e.g. `0.3.0`) rather than `:latest` to avoid unexpected updates. The image is multi-arch (amd64 + arm64) — Docker pulls the right variant automatically. See [Versioning & Compatibility](versioning.md).

Check: curl http://localhost:3000/health, curl http://localhost:3000/version. WebSocket API: ws://localhost:3000/ws (auth required).

## Docker Compose

docker-compose.yml:

```yaml
services:
  portal:
    image: getportal/sdk-daemon:0.3.0
    ports: ["3000:3000"]
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
```

.env: set PORTAL__AUTH__AUTH_TOKEN, PORTAL__NOSTR__PRIVATE_KEY; optionally PORTAL__WALLET__LN_BACKEND=nwc, PORTAL__WALLET__NWC__URL, PORTAL__NOSTR__RELAYS. Then docker compose up -d.

## Env vars

| Variable | Description |
|----------|-------------|
| `PORTAL__AUTH__AUTH_TOKEN` | API auth token (required). |
| `PORTAL__NOSTR__PRIVATE_KEY` | Nostr private key hex (required). |
| `PORTAL__WALLET__LN_BACKEND` | none, nwc, or breez. |
| `PORTAL__WALLET__NWC__URL` | NWC URL when ln_backend=nwc. |
| `PORTAL__WALLET__BREEZ__API_KEY` | Breez API key when ln_backend=breez. |
| `PORTAL__WALLET__BREEZ__MNEMONIC` | Breez mnemonic when ln_backend=breez. |
| `PORTAL__NOSTR__RELAYS` | Comma-separated relay URLs. |
| `PORTAL__NOSTR__SUBKEY_PROOF` | Proof for Nostr subkey delegation (optional). |

Full list: [Environment variables](environment-variables.md).

## Build image from repo

```bash
git clone https://github.com/PortalTechnologiesInc/lib.git
cd lib
docker build -t portal-rest:latest .
docker run -d -p 3000:3000 -e PORTAL__AUTH__AUTH_TOKEN=... -e PORTAL__NOSTR__PRIVATE_KEY=... portal-rest:latest
```

Or with Nix: nix build .#rest-docker then docker load &lt; result.

Use HTTPS and a reverse proxy in production; don’t commit secrets.

---

- [Environment variables](environment-variables.md) · [SDK](../sdk/installation.md) · [Building from source](building-from-source.md) · [Versioning](versioning.md)
