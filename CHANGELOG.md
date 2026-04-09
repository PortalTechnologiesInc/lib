# Changelog

All notable changes to Portal libraries will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## portal-rest (SDK Daemon)

### Unreleased

#### Added
- `MessageRouter` now exposes `SendOutcome` / `EventSendResult` from the underlying `portal` crate, giving relay delivery feedback per event. Not yet surfaced on the REST API; wiring will follow in a future release (#85)

---

### [0.4.1] - 2026-04-01

#### Changed
- `GET /info` now includes `version` and `git_commit` fields (previously only in `GET /version`). `GET /version` kept for backward compatibility.

#### Added
- **Age verification**: `POST /verification/sessions` creates a browser-based age verification session AND automatically starts listening for the verification token in a single call. Returns session info plus a `stream_id` to poll for the Cashu token result. Requires `[verification] api_key` in config. Relay URLs default to the `[nostr]` config if not provided.
- **Verification token request**: `POST /verification/token` requests a verification token from a user who already holds one (e.g. mobile app verified users). Returns a `stream_id` for async polling.
- TS SDK: `createVerificationSession()` now returns `AsyncOperation<CashuResponseStatus>` — use `poll()` or `done` to wait for the token. `requestVerificationToken()` for the mobile flow.
- Example: `examples/age-verification.js` — simplified single-call age verification flow
- **NIP-05 auto-registration**: if `PORTAL__PROFILE__NIP05` is set to a `@getportal.cc` address, the daemon registers it with the Portal profile service at startup (one-time, cached in `~/.portal-rest/nip05.registered`). Self-hosted domains are set in the Nostr profile only, no external call.

---

### [0.4.0] - 2026-03-17

#### Changed
- **WebSocket → REST**: All endpoints are now standard HTTP (POST/GET/PUT/DELETE); WebSocket API removed
- Async operations (key handshake, payments, invoices, cashu) now return `stream_id` immediately; events are delivered via webhooks or polling (`GET /events/:stream_id`)
- `GET /version` response now wrapped in standard `ApiResponse` envelope
- Profile is now set via `[profile]` config / env vars at startup; `SetProfile` command removed
- `FAKE_PAYMENTS` feature gated behind `#[cfg(debug_assertions)]`
- `/.well-known/nostr.json` is now a public endpoint (no auth required) for NIP-05 verification

#### Added
- **Webhook delivery**: HMAC-SHA256 signed POST (`X-Portal-Signature` header) to configured URL on every stream event
- **SQLite event persistence**: Events stored with WAL mode; startup recovery for in-flight `single_payment` streams
- `GET /info`: New authenticated endpoint returning server public key
- **TypeScript SDK**: `autoPollingIntervalMs` config option starts a background scheduler; `poll(op)` now typed — takes `AsyncOperation<T>`, returns `T`; `destroy()` stops the auto-poller; `webhookHandler()` for HMAC-verified webhook processing
- **Java SDK** (`portal-java-sdk`): Full rewrite from WebSocket to REST; `PortalClient` with `PortalClientConfig` supporting manual polling, auto-polling, and webhooks; all async methods return typed `AsyncOperation<T>`
- **OpenAPI spec**: `openapi.yaml` with all endpoints, request/response schemas, and `StreamIdResponse` for async operations
- **Documentation**: Full mdBook docs with HTTP/curl tabs on every page, REST API guide, interactive OpenAPI viewer (Redoc), updated environment variables reference
- **Examples**: Runnable Node.js examples (`auth.js`, `single-payment.js`, `profile.js`, `error-handling.js`) with `.env` support
- **Security**: Constant-time Bearer token comparison (`subtle::ConstantTimeEq`); constant-time webhook signature verification (`timingSafeEqual`)
- **Performance**: Shared `reqwest::Client` in `EventStore` for webhook delivery (no per-event allocation)

#### Fixed
- TypeScript SDK: `requestSinglePayment`, `requestPaymentRaw`, `requestRecurringPayment` now return correctly typed `AsyncOperation<T>` (previously returned raw `StreamEvent`)

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

#### Added
- `MessageRouter::add_conversation`, `add_conversation_with_relays` and `add_and_subscribe` now return `Vec<EventSendResult>` alongside their existing values, pairing each broadcasted Nostr event ID with a `SendOutcome` (`Delivered` / `Queued` / `Dropped`). Callers can now detect when a command is silently queued because no relay is connected (#85). Existing mobile app behavior is preserved — outcomes are currently ignored, ready to be wired into the UI when needed.

#### Changed
- `register_nip05()` now delegates to `portal::register_nip05()` (moved to `portal` crate). UniFFI bindings unchanged.

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
