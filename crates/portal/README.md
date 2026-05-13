# `portal`

**Nostr-first core for Portal:** typed payment and auth conversations, a relay-backed `MessageRouter`, and shared protocol models.

This crate is what `portal-rest`, `portal-sdk`, and `app` (in `portal-app`) all link against. It is not listed as a root workspace member in the top-level `Cargo.toml`, but it builds as a normal path dependency.

| Area | Notes |
|------|--------|
| `protocol` | Events, payments, JWT helpers, key handshake URLs |
| `conversation` | Per-flow state machines (single pay, invoices, Cashu, auth, …) |
| `router` | Listens on relays and dispatches incoming events |
| Features | `bindings` (UniFFI), `profile-service` (HTTP profile fetch for NIP-05) |

```toml
[dependencies]
portal = { path = "../portal", features = ["profile-service"] }
```

Full product docs: [portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/).
