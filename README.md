# Portal

**Nostr-based authentication and Lightning payments for your application.**

[![Documentation](https://img.shields.io/badge/docs-portaltechnologiesinc.github.io-blue)](https://portaltechnologiesinc.github.io/lib/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Portal is a Nostr-based authentication and payment SDK allowing applications to authenticate users and process payments through Nostr and Lightning Network.

---

## Table of contents

- [Features](#features)
- [Quick start](#quick-start)
- [Documentation](#documentation)
- [SDKs & API](#sdks--api)
- [Repository structure](#repository-structure)
- [License](#license)

---

## Features

| Area | Capabilities |
|------|--------------|
| **Authentication** | Nostr key handshake, main keys and subkeys, no passwords. Users sign in with a Nostr wallet (e.g. NWC). |
| **Payments** | Single and recurring Lightning payments; request invoices; Cashu mint, burn, request, and send. |
| **Identity** | Fetch and set Nostr profiles; NIP-05 resolution; issue and verify JWTs for session management. |
| **Platforms** | TypeScript/JavaScript and Java SDKs; React Native bindings; run the API yourself (Rust) or use a hosted endpoint. |

---

## Quick start

1. **Use an SDK** — Install the [TypeScript SDK](https://www.npmjs.com/package/portal-sdk) or [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk), connect to a Portal endpoint, and call the API with your auth token.
2. **Or run the API** — Self-host with Docker or [build from source](https://portaltechnologiesinc.github.io/lib/getting-started/building-from-source.html). Then use an SDK or connect to the WebSocket API.

**Example (Docker):**

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Use `ws://localhost:3000/ws` as the SDK WebSocket URL and your token for authentication. Health check: `curl http://localhost:3000/health`.

---

## Documentation

**Full documentation:** **[https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/)**

| Section | Contents |
|---------|----------|
| [Getting Started](https://portaltechnologiesinc.github.io/lib/getting-started/quick-start.html) | Quick Start, Docker, environment variables, building from source |
| [SDK](https://portaltechnologiesinc.github.io/lib/sdk/installation.html) | Installation, basic usage, configuration, error handling, API reference |
| [Guides](https://portaltechnologiesinc.github.io/lib/guides/authentication.html) | Authentication, single/recurring payments, profiles, Cashu, JWT, relays |
| [Resources](https://portaltechnologiesinc.github.io/lib/resources/faq.html) | FAQ, glossary, troubleshooting, contributing |

---

## SDKs & API

| Component | Description |
|-----------|-------------|
| **TypeScript / JavaScript** | [npm `portal-sdk`](https://www.npmjs.com/package/portal-sdk) — Node and browser; source in this repo under `crates/portal-rest/clients/ts`. |
| **Java** | [Portal Java SDK](https://github.com/PortalTechnologiesInc/java-sdk) — JVM (Java 17+); Gradle/Maven via JitPack. |
| **Portal API** | REST + WebSocket server in this repo (`crates/portal-rest`). Run it yourself or use a hosted endpoint. [API README](crates/portal-rest/README.md). |

---

## Repository structure

| Path | Description |
|------|-------------|
| `crates/portal-rest` | Portal API server (REST + WebSocket). [README](crates/portal-rest/README.md) for running and configuration. |
| `crates/portal-rest/clients/ts` | TypeScript SDK source. [README](crates/portal-rest/clients/ts/README.md). |
| `crates/portal` | Core Portal library (Rust): protocol, conversation handling, router. |
| `crates/portal-app`, `portal-sdk`, `portal-wallet`, `portal-cli`, `portal-rates` | App runtime, SDK core, wallet adapters, CLI tools, fiat rates. |
| `react-native` | React Native bindings for Portal. |
| `backend` | Example backend (TypeScript). |
| `docs` | Documentation source (mdBook). Published to [GitHub Pages](https://portaltechnologiesinc.github.io/lib/). |

---

## License

MIT where noted. See [LICENSE](LICENSE).
