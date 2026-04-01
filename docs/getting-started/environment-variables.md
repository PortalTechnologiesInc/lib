# Environment Variables

Portal reads `~/.portal-rest/config.toml` and overrides any value with environment variables using the format `PORTAL__<SECTION>__<KEY>` (double underscore separator).

## Config and env

- **Config file:** `~/.portal-rest/config.toml`. Full example: `crates/portal-rest/example.config.toml`.
- **Override:** `PORTAL__<SECTION>__<KEY>=value`. Example: `PORTAL__AUTH__AUTH_TOKEN=secret`.
- **Data:** wallet data (when `ln_backend=breez`) is stored under `~/.portal-rest/breez`.

## All settings

### Core

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `info.listen_port` | `PORTAL__INFO__LISTEN_PORT` | No | `3000` | HTTP port. |
| `auth.auth_token` | `PORTAL__AUTH__AUTH_TOKEN` | **Yes** | — | Bearer token for API access. |
| `nostr.private_key` | `PORTAL__NOSTR__PRIVATE_KEY` | **Yes** | — | Nostr private key (64 hex chars). |
| `nostr.relays` | `PORTAL__NOSTR__RELAYS` | No | `wss://relay.nostr.net, wss://relay.damus.io, wss://relay.getportal.cc` | Comma-separated relay URLs. |
| `nostr.subkey_proof` | `PORTAL__NOSTR__SUBKEY_PROOF` | No | — | Subkey delegation proof. |

### Database

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `database.path` | `PORTAL__DATABASE__PATH` | No | `portal-rest.db` | SQLite file path. Relative paths resolve under `~/.portal-rest/`. |

### Wallet

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `wallet.ln_backend` | `PORTAL__WALLET__LN_BACKEND` | No | `none` | `none`, `nwc`, or `breez`. |
| `wallet.nwc.url` | `PORTAL__WALLET__NWC__URL` | If `ln_backend=nwc` | — | NWC connection URL. |
| `wallet.breez.api_key` | `PORTAL__WALLET__BREEZ__API_KEY` | If `ln_backend=breez` | — | Breez API key. |
| `wallet.breez.mnemonic` | `PORTAL__WALLET__BREEZ__MNEMONIC` | If `ln_backend=breez` | — | Breez wallet mnemonic. |

### Webhooks

Webhooks are an alternative to polling — the daemon will `POST` events to your endpoint as they arrive.

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `webhook.url` | `PORTAL__WEBHOOK__URL` | No | — | URL to receive webhook events. |
| `webhook.secret` | `PORTAL__WEBHOOK__SECRET` | No | — | Shared secret for HMAC-SHA256 signatures (`X-Portal-Signature` header). |

When `webhook.secret` is set, each request includes an `X-Portal-Signature` header with the HMAC-SHA256 signature of the body. Verify it to authenticate incoming webhooks:

```python
import hmac, hashlib

def verify(secret: str, body: bytes, signature: str) -> bool:
    expected = hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(expected, signature)
```

### Profile

Publish your service's Nostr profile at startup. All fields are optional — omit the section to skip.

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `profile.name` | `PORTAL__PROFILE__NAME` | No | — | Username (no spaces). |
| `profile.display_name` | `PORTAL__PROFILE__DISPLAY_NAME` | No | — | Display name. |
| `profile.picture` | `PORTAL__PROFILE__PICTURE` | No | — | Avatar URL. |
| `profile.nip05` | `PORTAL__PROFILE__NIP05` | No | — | NIP-05 verified identifier. |

### Verification

Age verification via Portal's verification service. Required for `POST /verification/sessions`.

| Config key | Env var | Required | Default | Description |
|------------|---------|----------|---------|-------------|
| `verification.api_key` | `PORTAL__VERIFICATION__API_KEY` | No | — | API key for the Portal verification service. Get yours at [verify.getportal.cc](https://verify.getportal.cc). Required to use the verification endpoints. |

## Minimal setup

```bash
PORTAL__AUTH__AUTH_TOKEN=dev-token \
PORTAL__NOSTR__PRIVATE_KEY=your-64-char-hex-key \
portal-rest
```

Generate a token: `openssl rand -hex 32`  
Convert nsec to hex: `nak decode nsec1...`

## With Docker

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=my-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:0.4.0
```

Or use a `.env` file: `docker run --env-file .env ...`. Don't commit `.env`.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `401 Unauthorized` | Token must match `PORTAL__AUTH__AUTH_TOKEN`. |
| `Invalid Nostr key` | Must be 64 hex chars. Convert nsec: `nak decode nsec1...`. |
| Relays not connecting | Use `wss://` URLs; e.g. `wss://relay.damus.io`. |
| DB path error | Use an absolute path or ensure `~/.portal-rest/` exists. |

---

- [Docker](docker-deployment.md) · [Building from source](building-from-source.md) · [Quick start](quick-start.md)
