<div style="text-align: center;">
    <h1>
        <code>Portal</code>
    </h1>
    <h4>
        <a href="https://github.com/PortalTechnologiesInc">Project Homepage</a>
        <span> | </span>
        <a href="https://github.com/PortalTechnologiesInc/lib">Repository</a>
        <span> | </span>
        <a href="./donate.md">Become a supporter</a>
    </h4>
    <h2 style="font-size: 16px; font-weight: normal;">
        Portal is a Nostr-based authentication and payment SDK allowing applications to authenticate users and process payments through Nostr and Lightning Network.
    </h2>
</div>

## What is Portal?

Portal uses [Nostr](introduction/what-is-nostr.md) and the [Lightning Network](introduction/what-is-lightning.md) to provide:

- **Decentralized authentication** — Users sign in with Nostr keys; no passwords or email.
- **Lightning payments** — Single and recurring payments, real-time status.
- **Privacy-first** — No third parties, no data collection; direct peer-to-peer where possible.
- **Tickets & vouchers** — Issue Cashu ecash tokens to authenticated users.

## How to use Portal

Portal exposes a **standard HTTP REST API** — you can integrate from any language or platform.

1. **Run the Portal daemon** — self-host or develop locally: run `getportal/sdk-daemon` with Docker (see [Quick Start](getting-started/quick-start.md)).
2. **Call the REST API** — use any HTTP client (curl, Python, Go, Ruby, PHP…), or use the official SDKs for JavaScript/TypeScript and Java.
3. **Auth, payments, tickets** — generate auth URLs (users approve with Nostr wallet), request single or recurring Lightning payments, issue Cashu tokens.

## Integration options

| Option | When to use |
|--------|-------------|
| **HTTP / REST** | Any language — Python, Go, Ruby, PHP, Rust, etc. No SDK needed. |
| **JavaScript / TypeScript SDK** | Node.js and browser apps. Handles polling and webhooks automatically. |
| **Java SDK** | JVM apps. Same capabilities as the JS SDK. |

All options talk to the same REST API under the hood. The SDKs just add typed wrappers and auto-polling.

## Key features

- **Authentication** — Nostr key handshake, main keys and subkeys, no passwords.
- **Payments** — Single and recurring Lightning; Cashu mint/burn/request/send.
- **Profiles** — Fetch and set Nostr profiles; NIP-05.
- **Sessions** — Issue and verify JWTs for API access.
- **REST API** — Standard HTTP, [OpenAPI spec](sdk/api-reference-rest.md), any HTTP client.

## Getting started

- **[Quick Start](getting-started/quick-start.md)** — Get going in minutes with Docker + HTTP or an SDK.
- **[REST API](sdk/rest-api.md)** — Use Portal from any language over HTTP.
- **[OpenAPI Reference](sdk/api-reference-rest.md)** — Full interactive API reference.
- **[SDK](sdk/installation.md)** — Install the JavaScript or Java SDK.
- **[Docker](getting-started/docker-deployment.md)** — Run the Portal daemon with Docker.
- **[Guides](guides/authentication.md)** — Auth flow, payments, profiles, Cashu, JWT, relays.

## Docs overview

| Section | For |
|--------|-----|
| [Getting Started](getting-started/quick-start.md) | Quick Start, Docker, env vars, building from source. |
| [SDK & REST API](sdk/installation.md) | REST API, SDK install, usage, config, errors, OpenAPI reference. |
| [Guides](guides/authentication.md) | Auth, payments, profiles, Cashu, JWT, relays — with curl, JS, and Java examples. |
| [Resources](resources/faq.md) | FAQ, glossary, troubleshooting, contributing. |

## Open source

Portal is open source (MIT where noted). Contributions are welcome.

**Next:** [Quick Start](getting-started/quick-start.md)
