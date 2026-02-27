# Portal

[![Documentation](https://img.shields.io/badge/docs-portaltechnologiesinc.github.io-blue)](https://portaltechnologiesinc.github.io/lib/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-orange)](CHANGELOG.md)
[![Docker](https://img.shields.io/docker/v/getportal/sdk-daemon?label=sdk-daemon&color=blue)](https://hub.docker.com/r/getportal/sdk-daemon)

**Portal is the identity and payment layer for Bitcoin-native applications — no accounts, no KYC, no payment processor.**

Users interact through the Portal mobile app using their Nostr identity. You run a single Docker container and talk to it with our SDK.

---

## How it works

```
Your backend  ←—WebSocket—→  sdk-daemon  ←—Nostr relays—→  Portal app (user's phone)
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
  getportal/sdk-daemon:0.3.0
```

**2. Install the SDK**

```bash
npm install portal-sdk          # TypeScript / JavaScript
```

```groovy
// Java (Gradle) — via JitPack
implementation 'com.github.PortalTechnologiesInc:java-sdk:0.3.0'
```

**3. Connect and authenticate a user**

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client.connect();
await client.authenticate('your-secret-token');

const url = await client.newKeyHandshakeUrl((userKey) => {
  console.log('User authenticated:', userKey);
});
// Show `url` as a QR code to the user
```

---

## SDK versions

| SDK | Version | Install |
|-----|---------|---------|
| TypeScript / JavaScript | `0.3.0` | `npm install portal-sdk` |
| Java | `0.3.0` | [JitPack](https://jitpack.io/#PortalTechnologiesInc/java-sdk) |
| Docker image | `0.3.0` | `docker pull getportal/sdk-daemon:0.3.0` |

The SDK `major.minor` version must match the daemon. Patch versions are independent. See [Versioning & Compatibility](https://portaltechnologiesinc.github.io/lib/getting-started/versioning.html).

---

## What you can do

- **Authenticate users** — passwordless login via Nostr identity
- **Request payments** — single, recurring, or invoice-based; BTC (sats) or fiat (EUR, USD, and [more](https://portaltechnologiesinc.github.io/lib/sdk/api-reference.html))
- **Issue JWTs** — signed by the user's Nostr key, verifiable server-side
- **Cashu tokens** — mint, burn, and transfer ecash
- **NWC wallet** — connect any NWC-compatible wallet for outbound payments

---

## This repository

| Package | Description |
|---------|-------------|
| `portal-rest` | SDK Daemon — HTTP + WebSocket API server |
| `portal` | Core Nostr protocol and conversation logic |
| `portal-app` | App runtime and wallet integration |
| `portal-wallet` | Wallet backends (NWC, Breez) |
| `portal-rates` | Fiat/BTC exchange rates |
| `clients/ts` | TypeScript SDK source |
| `portal-cli` | CLI tools for development and testing |

---

## Documentation

Full docs at **[portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/)** — guides for authentication, payments, Cashu, JWT, relay setup, Docker deployment, and more.

---

**License:** MIT · [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk) · [Demo](https://github.com/PortalTechnologiesInc/portal-demo)
