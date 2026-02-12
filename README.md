# Portal

[![Documentation](https://img.shields.io/badge/docs-portaltechnologiesinc.github.io-blue)](https://portaltechnologiesinc.github.io/lib/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Portal is a Nostr-based authentication and payment SDK allowing applications to authenticate users and process payments through Nostr and Lightning Network.

**Documentation:** [portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/)

---

## Repository

| Crate / package | Description |
|-----------------|-------------|
| portal-rest | API server (REST + WebSocket). Run with Docker or from source. |
| portal | Core protocol and conversation logic. |
| portal-app | App runtime and wallet integration. |
| portal-cli | CLI tools (JWT, invoices, Cashu, etc.). |
| portal-wallet | Wallet backends (NWC, Breez). |
| portal-rates | Fiat exchange rates for BTC/sats. |
| clients/ts | TypeScript SDK ([npm](https://www.npmjs.com/package/portal-sdk)). |
| Java SDK | [PortalTechnologiesInc/java-sdk](https://github.com/PortalTechnologiesInc/java-sdk). |

## Quick start

**Run the API (Docker):**

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Then use `ws://localhost:3000/ws` and your token with the [TypeScript](https://www.npmjs.com/package/portal-sdk) or [Java](https://github.com/PortalTechnologiesInc/java-sdk) SDK. See [Quick Start](https://portaltechnologiesinc.github.io/lib/getting-started/quick-start.html) in the docs.

**Build from source:** [Building from source](https://portaltechnologiesinc.github.io/lib/getting-started/building-from-source.html) — `cargo build --package portal-rest --release` or `nix build .#rest`.

---

**License:** MIT — [LICENSE](LICENSE)
