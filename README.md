<div align="center">

# Portal

**Identity and payments for Bitcoin-native applications.**

No accounts. No KYC. No payment processor.

[📖 **Documentation**](https://portaltechnologiesinc.github.io/lib/) · [🚀 **Get Started**](https://hub.getportal.cc) · [🐳 Docker](https://hub.docker.com/r/getportal/sdk-daemon)

[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-orange)](CHANGELOG.md)

</div>

---

Portal lets you add **age verification**, **authentication**, and **payments** to your app — without collecting personal data.

> **New here?** Start with the [documentation](https://portaltechnologiesinc.github.io/lib/) or create a free instance on [PortalHub](https://hub.getportal.cc).

## What can you build?

- **Age verification** — verify users' age for compliance, no personal data stored → [Guide](https://portaltechnologiesinc.github.io/lib/age-verification/getting-started.html)
- **Authentication** — passwordless login via the Portal app → [Guide](https://portaltechnologiesinc.github.io/lib/platform/authentication.html)
- **Payments** — single, recurring, invoice-based; BTC or fiat → [Guide](https://portaltechnologiesinc.github.io/lib/platform/single-payments.html)
- **Digital tickets** — issue and verify Cashu tokens → [Guide](https://portaltechnologiesinc.github.io/lib/platform/cashu-tokens.html)

## Quick start

The fastest way: create an instance on [PortalHub](https://hub.getportal.cc) — no servers needed.

Or self-host:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$(openssl rand -hex 32) \
  -e PORTAL__NOSTR__PRIVATE_KEY=<your-64-char-hex-key> \
  getportal/sdk-daemon:0.4.1
```

## SDKs

| Language | Install |
|----------|---------|
| TypeScript / JS | `npm install portal-sdk` |
| Java | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |
| Any language | [REST API](https://portaltechnologiesinc.github.io/lib/sdk/rest-api.html) — no SDK needed |

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
