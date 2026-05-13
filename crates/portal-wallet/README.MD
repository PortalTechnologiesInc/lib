# `portal-wallet`

**Lightning backends behind one trait:** pay invoices, issue invoices, read balance—either over **Nostr Wallet Connect (NWC)** or **Breez Spark** (`breez-sdk-spark`).

| Type | When to use |
|------|-------------|
| `NwcWallet` | You already have a `nostr+walletconnect://…` URI |
| `BreezSparkWallet` | You run inside Breez’s Spark stack with API key + mnemonic |

```rust
use portal_wallet::{PortalWallet, NwcWallet};
// Implementations are async; see `PortalWallet` in src/lib.rs.
```

`portal-rest` wires these up for server-side pay/invoice paths; `portal-app-demo` can attach a wallet per demo session.

## Build note (protobuf)

`breez-sdk-spark` pulls in `tonic-build` / `prost-build`. You need the Protocol Buffers compiler and development files available on your machine (so `.proto` generation succeeds).

Example on Fedora:

```bash
sudo dnf install protobuf-compiler protobuf-devel
```

Adjust for your distro (Debian/Ubuntu often use `protobuf-compiler` and `libprotobuf-dev`).
