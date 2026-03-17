# Changelog

All notable changes to Portal libraries will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## portal-rest (SDK Daemon)

### Unreleased

---

### [0.4.0] - 2026-03-17

#### Changed
- **WebSocket → REST**: All endpoints are now standard HTTP (POST/GET/PUT/DELETE); WebSocket API removed
- Async operations (key handshake, payments, invoices, cashu) now return `stream_id` immediately; events are delivered via webhooks or polling (`GET /events/:stream_id`)
- `GET /version` response now wrapped in standard `ApiResponse` envelope
- Profile is now set via `[profile]` config / env vars at startup; `SetProfile` command removed
- `FAKE_PAYMENTS` feature gated behind `#[cfg(debug_assertions)]`

#### Added
- **Webhook delivery**: HMAC-SHA256 signed POST to configured URL on every stream event
- **SQLite event persistence**: Events stored with startup recovery for in-flight streams
- `GET /info`: New authenticated endpoint returning server public key
- **TypeScript SDK**: `autoPollingIntervalMs` config option starts a background scheduler that resolves `done` promises automatically; `poll(op)` now typed — takes `AsyncOperation<T>`, returns `T`
- **Java SDK** (`portal-java-sdk`): Full rewrite from WebSocket to REST; `PortalClient` with `PortalClientConfig` supporting manual polling, auto-polling, and webhooks; all async methods return typed `AsyncOperation<T>`

#### Removed
- WebSocket API (`/ws` endpoint)
- `SetProfile` / `ListenClosedRecurringPayment` commands

---

### [0.3.0] - 2026-02-26

#### Changed
- `RequestInvoice` command now uses slim `RequestInvoiceParams`: `current_exchange_rate` removed (server computes it); `request_id` is now optional (defaults to command ID if omitted)
- SDK versions (portal-sdk npm, portal-java-sdk) are now aligned with portal-rest version for easy compatibility tracking

#### Fixed
- Validate `RequestInvoice` response: invoice amount must match expected amount (computed from FIAT conversion). Allows 1 msat tolerance for rounding errors (#157)

---

### [0.2.0] - 2026-02-17

#### Added
- `PayInvoice` command
- `GetWalletInfo` command
- `RequestSinglePayment` accepts an optional `request_id` parameter

#### Changed
- Default Breez storage directory changed to `~/.portal-rest/breez`
- Removed unused dependencies (`dotenv`, `bitflags`, `android_logger`, `async`); workspace deps centralized and reorganized
- Updated Breez SDK

#### Fixed
- Cargo warnings cleanup

---

### [0.1.0] - 2026-02-09

First release — Docker image available at [`getportal/sdk-daemon`](https://hub.docker.com/r/getportal/sdk-daemon).

---

## portal-app (App Library)

### Unreleased

---

### [0.4.13] - 2026-01-27

---

### [0.4.12] - 2025-12-08

---

### [0.4.11] - 2025-12-02

---

### [0.4.10] - 2025-11-11

---

### [0.4.9] - 2025-11-04

---

### [0.4.8] - 2025-10-30

---

### [0.4.7] - 2025-10-07

---

### [0.4.6] - 2025-09-23

---

### [0.4.5] - 2025-09-18

Releasing a new version to include iOS build on npm. No relevant updates are in this release.

---

### [0.4.4] - 2025-09-10

- Fixing profile conversation expiring check

---

### [0.4.3] - 2025-09-04

- Only set preferred relays in key handshake urls
- Expose missing expiration

---

### [0.4.2] - 2025-08-22

- Update nostr git dep to include fixes for NWC

---

### [0.4.1] - 2025-08-13

- Connect relays in URL
- LNURL parsing
- Reconnect relay NWC

---

### [0.4.0] - 2025-08-01

- Update nostr to 0.43

---

### [0.3.1] - 2025-08-01

- Make the relay reconnection parallel and not sequential
- Disable automatic relay reconnection

---

### [0.3.0] - 2025-08-01

- Use binary cache to build the library
- Send multiple notifications for single payments
