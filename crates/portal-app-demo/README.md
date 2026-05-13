# `portal-app-demo`

**Local multi-session lab:** an Axum server where each browser (or API client) spins up a `PortalApp` actor with its own keys and optional NWC wallet. Built for protocol testing, not production.

```bash
cargo run -p portal-app-demo
```

Open `http://127.0.0.1:3030` (port from config). Sessions are created with `POST /api/session` (mnemonic or `nsec` in the JSON body); each session gets SSE streams for payment and invoice events and buttons to accept, reject, or fulfill flows.

## Config

Path: **`~/.portal-app-demo/config.toml`**. On first run the app can materialize it from `example.config.toml` in this crate.

| Section | Purpose |
|---------|---------|
| `[info]` | `listen_port` (default `3030`) |
| `[nostr]` | `default_relays` for new sessions |
| `[session]` | `max_sessions`, `ttl_secs` caps per in-memory actor pool |

Override any key with env vars: `PORTAL_APP_DEMO__<SECTION>__<KEY>=value` (double underscores).

Per-session payment backends (NWC / Breez) are chosen in the session-creation API or UI—not in the global TOML. Breez-related data for a session lives under the demo’s data directory as implemented in `app` / wallet code paths.

## How it differs from `portal-rest`

| | `portal-app-demo` | `portal-rest` |
|--|-------------------|----------------|
| Audience | Developers stepping through flows | Integrators and apps |
| Auth | Per-session keys you supply | Server Bearer token + operator keys |
| Scale | In-memory actors, TTL-bound | Production daemon model |

For shipping to users, use the REST daemon and the [hosted hub](https://hub.getportal.cc) or your own `portal-rest` deployment.
