<div align="center">

# Portal

**Ship login and Lightning payments in Bitcoin apps—without accounts, KYC, or a card processor.**

[Documentation](https://portaltechnologiesinc.github.io/lib/) · [PortalHub](https://hub.getportal.cc) · [Docker image](https://hub.docker.com/r/getportal/sdk-daemon)

[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-orange)](CHANGELOG.md)

</div>

---

Users prove identity with keys. You charge in BTC (or priced in fiat) over Nostr. This repo is the **Rust workspace** behind the REST daemon, core protocol, and native building blocks.

| You want | Portal gives you |
|----------|------------------|
| Passwordless sign-in | Key handshake + Nostr; [auth guide](https://portaltechnologiesinc.github.io/lib/platform/authentication.html) |
| One-off and recurring pay | Single and subscription flows; [payments](https://portaltechnologiesinc.github.io/lib/platform/single-payments.html) |
| Tickets / tokens | Cashu paths; [tokens](https://portaltechnologiesinc.github.io/lib/platform/cashu-tokens.html) |

## Quick start

Fastest path: spin up a hosted instance on [PortalHub](https://hub.getportal.cc)—no server to run.

Self-host the REST daemon:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32) \
  -e PORTAL__NOSTR__PRIVATE_KEY=<your-64-char-hex-key> \
  getportal/sdk-daemon:0.4.2
```

Then `curl http://localhost:3000/health` should return `OK`.

## Client SDKs

| Where | Install / link |
|-------|----------------|
| TypeScript / JavaScript | `npm install portal-sdk` |
| Java | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |
| Anything with HTTP | [REST API](https://portaltechnologiesinc.github.io/lib/sdk/rest-api.html) |

## Workspace layout

Rust crates live under `crates/`. The `portal` crate is not a top-level workspace member but is the shared core every service binary depends on.

| Crate | What it is |
|-------|------------|
| [`portal`](crates/portal) | Protocol types, Nostr conversations, message router |
| [`portal-rest`](crates/portal-rest) | `rest` binary: Bearer auth, streaming events, webhooks |
| [`portal-app`](crates/portal-app) | Package **`app`**: UniFFI staticlib + runtime (wallet, relays, payment UI hooks) |
| [`portal-wallet`](crates/portal-wallet) | `PortalWallet` implementations: NWC, Breez Spark |
| [`portal-sdk`](crates/portal-sdk) | Async SDK: relay pool + high-level send/receive helpers |
| [`portal-rates`](crates/portal-rates) | Fiat/BTC rates (multi-source, BlueWallet-style logic) |
| [`portal-macros`](crates/portal-macros) | Build-time `fetch_git_hash!` and related macros |
| [`portal-cli`](crates/portal-cli) | Small binaries for manual protocol and integration checks |
| [`portal-app-demo`](crates/portal-app-demo) | Local Axum demo: multi-session HTTP API over `app` |
| [`portal-rest/clients/ts`](crates/portal-rest/clients/ts) | TypeScript client |

## License

MIT — see [LICENSE](LICENSE).
