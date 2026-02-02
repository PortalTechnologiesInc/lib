# Welcome to Portal

**Published docs:** [https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/)

Portal is a **Nostr-based authentication and payment SDK** allowing applications to authenticate users and process payments through Nostr and Lightning Network.

You can integrate via the **official SDKs** (TypeScript, Java) or run the Portal API yourself and use the protocol directly.

## What is Portal?

Portal uses [Nostr](introduction/what-is-nostr.md) and the [Lightning Network](introduction/what-is-lightning.md) to provide:

- **Decentralized authentication** — Users sign in with Nostr keys; no passwords or email.
- **Lightning payments** — Single and recurring payments, real-time status.
- **Privacy-first** — No third parties, no data collection; direct peer-to-peer where possible.
- **Tickets & vouchers** — Issue Cashu ecash tokens to authenticated users.

## How to use Portal

1. **Integrate with an SDK** — Use the [JavaScript SDK](sdk/installation.md) or [Java SDK](sdk/installation.md) (same docs, switch tab): connect to a Portal endpoint with an auth token and call the API.
2. **Or run the API** — Self-host or develop locally: run the Portal API (Docker or [build from source](getting-started/building-from-source.md)); then use an SDK or connect to the WebSocket API.
3. **Auth, payments, tickets** — Generate auth URLs (users approve with Nostr wallet); request single or recurring Lightning payments; issue Cashu tokens.

## Key features

- **Authentication** — Nostr key handshake, main keys and subkeys, no passwords.
- **Payments** — Single and recurring Lightning; Cashu mint/burn/request/send.
- **Profiles** — Fetch and set Nostr profiles; NIP-05.
- **Sessions** — Issue and verify JWTs for API access.
- **SDKs** — TypeScript/JavaScript and JVM; React Native bindings.

## Getting started

- **[Quick Start](getting-started/quick-start.md)** — Get going with the SDK (JavaScript or Java) or run Portal with Docker.
- **[SDK](sdk/installation.md)** — Install, [basic usage](sdk/basic-usage.md), [configuration](sdk/configuration.md), [error handling](sdk/error-handling.md).
- **[Docker](getting-started/docker-deployment.md)** — Run the Portal API with Docker.
- **[Building from source](getting-started/building-from-source.md)** — Build and run the Rust project.
- **[Guides](guides/authentication.md)** — Auth flow, payments, profiles, Cashu, JWT, relays.

## Docs overview

| Section | For |
|--------|-----|
| [Getting Started](getting-started/quick-start.md) | Quick Start, Docker, env vars, building from source. |
| [SDK](sdk/installation.md) | SDK install, usage, config, errors. |
| [Guides](guides/authentication.md) | Auth, payments, profiles, Cashu, JWT, relays. |
| [Resources](resources/faq.md) | FAQ, glossary, troubleshooting, contributing. |

## Open source

Portal is open source (MIT where noted). Contributions are welcome.

**Next:** [Quick Start](getting-started/quick-start.md)
