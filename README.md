# Portal

Portal is a **Nostr-based authentication and payment SDK** allowing applications to authenticate users and process payments through Nostr and Lightning Network.

This repo contains the Rust implementation, the REST/WebSocket API (`portal-rest`), and official SDKs (TypeScript, Java). 

## Use the SDK

Install the SDK for your language and connect to a Portal endpoint (and auth token).

### TypeScript / JavaScript

```bash
npm install portal-sdk
```

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: process.env.PORTAL_URL ?? 'ws://localhost:3000/ws',
});

await client.connect();
await client.authenticate(process.env.PORTAL_AUTH_TOKEN!);

// e.g. generate auth URL for a user (Nostr key handshake)
const url = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('User authenticated:', mainKey);
});
console.log('Share this URL with your user:', url);
```

**Full SDK docs:** [TypeScript SDK](crates/portal-rest/clients/ts/README.md) — quick start, workflows, API reference, error handling.

### Java

[Java client](https://github.com/PortalTechnologiesInc/jvm-client) — connect to a Portal endpoint and authenticate with a token; same pattern as above.

### Documentation

- **[Quick Start](docs/getting-started/quick-start.md)** — Get going with the SDK or run Portal with Docker.
- **[TypeScript SDK](docs/sdk/installation.md)** — Install, [basic usage](docs/sdk/basic-usage.md), [configuration](docs/sdk/configuration.md), [error handling](docs/sdk/error-handling.md).
- **[Guides](docs/guides/authentication.md)** — Auth, payments, profiles, Cashu, JWT, relays.

---

## Features

- **Authentication** — Nostr key handshake; main keys and subkeys; no passwords.
- **Payments** — Single and recurring Lightning; real-time status; Cashu mint/burn/request/send.
- **Profiles** — Fetch and set Nostr profiles; NIP-05.
- **Sessions** — Issue and verify JWTs for API access.
- **SDKs** — TypeScript/JavaScript, Java.

---

## Repository structure

| Path | Description |
|------|-------------|
| `crates/portal-rest` | Portal API (REST + WebSocket); [SDK docs and Run Portal](crates/portal-rest/README.md). |
| `crates/portal-rest/clients/ts` | [TypeScript SDK](crates/portal-rest/clients/ts/README.md). |
| `crates/portal` | Core Portal library (Rust). |
| `crates/portal-app` | API exposed to the app. |
| `crates/portal-sdk` | Core SDK implementation. |
| `crates/portal-wallet` | Wallets (NWC, Breez). |
| `crates/portal-cli` | CLI tools. |
| `crates/portal-rates` | Exchange rates. |
| `react-native` | React Native bindings. |
| `backend` | Example backend. |
| `docs` | [Documentation](docs/README.md) (Quick Start, SDK, guides). |

---

## Building from source

**Prerequisites:** Rust (latest stable), Node.js/npm (for TypeScript SDK).

```bash
cargo build --release
```

To run the Portal API server locally (e.g. for development): see [portal-rest](crates/portal-rest/README.md#run-portal-when-you-need-your-own-instance) and [Building from source](docs/getting-started/building-from-source.md).

---

## API documentation

- **Using the SDK:** [TypeScript SDK README](crates/portal-rest/clients/ts/README.md) and [docs](docs/README.md).
- **Raw API (advanced):** [portal-rest README](crates/portal-rest/README.md#api-reference-advanced) — WebSocket commands when not using an SDK.

---

## License

MIT License, except for the app library. See [LICENSE](LICENSE).
