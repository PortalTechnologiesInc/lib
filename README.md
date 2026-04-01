# Portal

[![Documentation](https://img.shields.io/badge/docs-portaltechnologiesinc.github.io-blue)](https://portaltechnologiesinc.github.io/lib/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-orange)](CHANGELOG.md)
[![Docker](https://img.shields.io/docker/v/getportal/sdk-daemon?label=sdk-daemon&color=blue)](https://hub.docker.com/r/getportal/sdk-daemon)

**Portal is the identity and payment layer for Bitcoin-native applications — no accounts, no KYC, no payment processor.**

Users interact through the Portal mobile app using their Nostr identity. You run a single Docker container and call its REST API from any language.

---

## How it works

```
Your backend  ←—REST API—→  sdk-daemon  ←—Nostr relays—→  Portal app (user's phone)
```

1. Your backend asks sdk-daemon for a handshake URL → show it as a QR code
2. User scans with Portal app → you receive their Nostr public key
3. Your backend requests a payment (sats or fiat) → user approves in app → Lightning settlement

---

## Quick start

**1. Run sdk-daemon**

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:0.4.0
```

**2. Call the REST API**

```bash
# Get a handshake URL — show it to the user as a QR code
curl -s -X POST http://localhost:3000/key-handshake \
  -H "Authorization: Bearer your-secret-token" \
  -H "Content-Type: application/json" \
  -d '{}' | jq .

# → { "stream_id": "abc123", "url": "nostr+walletconnect://..." }

# Poll for the user's public key (repeat until events arrive)
curl -s "http://localhost:3000/events/abc123?after=0" \
  -H "Authorization: Bearer your-secret-token" | jq .
```

Any language works — Python, Go, Ruby, PHP, Java, TypeScript. No SDK required.

---

## What you can do

- **Authenticate users** — passwordless login via Nostr identity
- **Age verification** — browser-based identity verification with cryptographic proof ([guide](https://portaltechnologiesinc.github.io/lib/guides/age-verification.html))
- **Request payments** — single, recurring, or invoice-based; BTC (sats) or fiat (EUR, USD, and [more](https://portaltechnologiesinc.github.io/lib/sdk/api-reference.html))
- **Issue JWTs** — signed by the user's Nostr key, verifiable server-side
- **Cashu tokens** — mint, burn, and transfer ecash
- **NWC wallet** — connect any NWC-compatible wallet for outbound payments

---

## Documentation

Full docs at **[portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/)** — REST API reference, curl examples, guides for authentication, payments, Cashu, JWT, Docker deployment, and more.

---

## Official SDKs

For JavaScript/TypeScript and Java, typed SDKs are available. They wrap the same REST API with auto-polling, webhook handling, and typed responses.

| SDK | Version | Install |
|-----|---------|---------|
| TypeScript / JavaScript | `0.4.0` | `npm install portal-sdk` |
| Java | `0.4.0` | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |

The SDK `major.minor` version must match the daemon. See [Versioning & Compatibility](https://portaltechnologiesinc.github.io/lib/getting-started/versioning.html).

---

## This repository

| Package | Description |
|---------|-------------|
| `portal-rest` | SDK Daemon — REST API server |
| `portal` | Core Nostr protocol and conversation logic |
| `portal-app` | App runtime and wallet integration |
| `portal-wallet` | Wallet backends (NWC, Breez) |
| `portal-rates` | Fiat/BTC exchange rates |
| `clients/ts` | TypeScript SDK source |
| `portal-cli` | CLI tools for development and testing |

---

**License:** MIT · [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk) · [Demo](https://github.com/PortalTechnologiesInc/portal-demo)
