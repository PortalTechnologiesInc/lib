<div align="center">

# Portal

**Identity and payments for Bitcoin-native applications.**

No accounts. No KYC. No payment processor.

[![Docs](https://img.shields.io/badge/docs-portal-blue)](https://portaltechnologiesinc.github.io/lib/)
[![Docker](https://img.shields.io/docker/v/getportal/sdk-daemon?label=docker&color=blue)](https://hub.docker.com/r/getportal/sdk-daemon)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-orange)](CHANGELOG.md)

</div>

## About

Portal lets users authenticate and pay using their Nostr identity through the Portal mobile app. You run a single Docker container (`sdk-daemon`) and call its REST API from any language.

```
Your backend  ←— REST API —→  sdk-daemon  ←— Nostr —→  Portal app
```

## Features

- **Authentication** — passwordless login via Nostr
- **Age verification** — browser-based identity verification with cryptographic proof
- **Payments** — single, recurring, invoice-based; BTC or fiat
- **JWTs** — signed by the user's Nostr key
- **Cashu tokens** — mint, burn, and transfer ecash
- **Wallets** — NWC or Breez for outbound payments

## Quick start

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32) \
  -e PORTAL__NOSTR__PRIVATE_KEY=<your-64-char-hex-key> \
  getportal/sdk-daemon:0.4.1
```

Then follow the [Quick Start guide](https://portaltechnologiesinc.github.io/lib/getting-started/quick-start.html) or jump to the [REST API reference](https://portaltechnologiesinc.github.io/lib/sdk/rest-api.html).

## SDKs

| Language | Install |
|----------|---------|
| TypeScript / JS | `npm install portal-sdk` |
| Java | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |
| Any language | [REST API](https://portaltechnologiesinc.github.io/lib/sdk/rest-api.html) — no SDK needed |

## Documentation

Full docs: **[portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/)**

Guides: [Authentication](https://portaltechnologiesinc.github.io/lib/guides/authentication.html) · [Age Verification](https://portaltechnologiesinc.github.io/lib/guides/age-verification.html) · [Payments](https://portaltechnologiesinc.github.io/lib/guides/single-payments.html) · [Cashu](https://portaltechnologiesinc.github.io/lib/guides/cashu-tokens.html) · [JWT](https://portaltechnologiesinc.github.io/lib/guides/jwt-tokens.html) · [Docker](https://portaltechnologiesinc.github.io/lib/getting-started/docker-deployment.html)

## Repository structure

| Crate | Description |
|-------|-------------|
| `portal-rest` | REST API daemon |
| `portal` | Core protocol and conversation logic |
| `portal-app` | App runtime and wallet integration |
| `portal-wallet` | Wallet backends (NWC, Breez) |
| `portal-rates` | Fiat/BTC exchange rates |
| `clients/ts` | TypeScript SDK |
| `portal-cli` | Dev/testing CLI tools |

## License

MIT — see [LICENSE](LICENSE)
