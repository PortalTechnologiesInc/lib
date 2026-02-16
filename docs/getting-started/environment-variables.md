# Environment Variables

Portal (portal-rest) reads ~/.portal-rest/config.toml and overrides with PORTAL__&lt;SECTION&gt;__&lt;KEY&gt;.

## Config and env

- **Config:** ~/.portal-rest/config.toml. Example: crates/portal-rest/example.config.toml.
- **Override:** PORTAL__&lt;SECTION&gt;__&lt;KEY&gt;=value (double underscore). Example: PORTAL__AUTH__AUTH_TOKEN=secret.
- **Data:** Breez wallet data (when `ln_backend=breez`) is stored under ~/.portal-rest/breez.

| Config key | Env | Description |
|------------|-----|-------------|
| info.listen_port | `PORTAL__INFO__LISTEN_PORT` | API port (default 3000). |
| auth.auth_token | `PORTAL__AUTH__AUTH_TOKEN` | API auth token. Required. |
| nostr.private_key | `PORTAL__NOSTR__PRIVATE_KEY` | Nostr key (hex). Required. |
| nostr.relays | `PORTAL__NOSTR__RELAYS` | Comma-separated relay URLs. |
| nostr.subkey_proof | `PORTAL__NOSTR__SUBKEY_PROOF` | Subkey delegation proof. |
| wallet.ln_backend | `PORTAL__WALLET__LN_BACKEND` | `none`, `nwc`, or `breez`. |
| wallet.nwc.url | `PORTAL__WALLET__NWC__URL` | NWC URL (when `ln_backend=nwc`). |
| wallet.breez.api_key | `PORTAL__WALLET__BREEZ__API_KEY` | Breez API key (when `ln_backend=breez`). |
| wallet.breez.mnemonic | `PORTAL__WALLET__BREEZ__MNEMONIC` | Breez mnemonic (when `ln_backend=breez`). |

## Minimal run

```bash
PORTAL__AUTH__AUTH_TOKEN=dev-token \
PORTAL__NOSTR__PRIVATE_KEY=your-key-hex \
portal-rest
```

Generate token: openssl rand -hex 32. Nostr key: hex (64 chars); from nsec: nak decode nsec1...

## With Docker

Pass env or use .env and docker run --env-file .env (or env_file in Compose). Don’t commit .env.

## Troubleshooting

- **Auth failed:** Token in client must match PORTAL__AUTH__AUTH_TOKEN.
- **Invalid key:** Must be 64 hex chars.
- **Relays:** Use wss:// URLs; e.g. wss://relay.damus.io, wss://relay.snort.social.

---

- [Docker](docker-deployment.md) · [Building from source](building-from-source.md) · [Quick start](quick-start.md)
