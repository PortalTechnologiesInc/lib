# `portal-app` (package `app`)

**Mobile-oriented Portal runtime:** static library + Rust API that wires `portal` to relays, optional NWC, CDK wallet bits, and app-level payment/auth listeners.

Published Cargo name is **`app`** (see `Cargo.toml`). Workspace folder name is `portal-app`.

| Output | Role |
|--------|------|
| `staticlib` + `lib` | UniFFI-friendly embedding in iOS/Android shells |
| Runtime types | `PortalApp`, mnemonic/nsec identity, relay status callbacks |

Typical dependency from another crate in this repo:

```toml
[dependencies]
app = { path = "../portal-app", features = [] }
```

Feature flags follow `portal` (e.g. `profile-service` on the `portal` dependency inside this crate). For end-to-end behavior and HTTP integration, use **`portal-rest`** or the **`portal-app-demo`** harness.
