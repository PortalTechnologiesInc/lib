# `portal-sdk`

**In-process Portal client:** connect to relays, run the shared `MessageRouter`, and drive auth, payments, invoices, Cashu, and profile flows from async Rust—without standing up `portal-rest`.

| Good fit | Less ideal |
|----------|------------|
| Services or tests that already speak Nostr | Public HTTP API for arbitrary languages (use REST + npm SDK instead) |

```toml
[dependencies]
portal-sdk = { path = "../portal-sdk" }
```

Entry type is `PortalSDK::new(keypair, relays)` (see `src/lib.rs`). Errors and conversation types re-export patterns from the `portal` crate.

For operators and integrators, the [published docs](https://portaltechnologiesinc.github.io/lib/) still center on the REST daemon and TypeScript client.
