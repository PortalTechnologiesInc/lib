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

The fastest way to get started is with [PortalHub](https://hub.getportal.cc) — create an instance in seconds, no servers needed.

Or self-host with Docker:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32) \
  -e PORTAL__NOSTR__PRIVATE_KEY=<your-64-char-hex-key> \
  getportal/sdk-daemon:0.4.1
```

Then follow the [Age Verification guide](https://portaltechnologiesinc.github.io/lib/age-verification/getting-started.html) or the [Platform guide](https://portaltechnologiesinc.github.io/lib/platform/getting-started.html).

## SDKs

| Language | Install |
|----------|---------|
| TypeScript / JS | `npm install portal-sdk` |
| Java | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |
| Any language | [REST API](https://portaltechnologiesinc.github.io/lib/sdk/rest-api.html) — no SDK needed |

## Documentation

Full docs: **[portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/)**

Guides: [Age Verification](https://portaltechnologiesinc.github.io/lib/age-verification/getting-started.html) · [Authentication](https://portaltechnologiesinc.github.io/lib/platform/authentication.html) · [Payments](https://portaltechnologiesinc.github.io/lib/platform/single-payments.html) · [Cashu](https://portaltechnologiesinc.github.io/lib/platform/cashu-tokens.html) · [JWT](https://portaltechnologiesinc.github.io/lib/platform/jwt-tokens.html) · [Docker](https://portaltechnologiesinc.github.io/lib/advanced/docker-deployment.html)

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
