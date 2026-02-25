# portal-app-demo

Minimal demo app for **protocol testing**: single instance from config, receive a single payment request, Accept or Reject. Uses **portal-app**. Optional payment wallet (NWC or Breez) from config; Breez data is stored under `~/.portal-app-demo/breez`.

## Config

One app instance. Config file: **`~/.portal-app-demo/config.toml`** (created on first run from `example.config.toml`).

- **Identity**: `[identity]` — set `mnemonic` (12 words) or `nsec`.
- **Relays**: `[nostr]` — `relays = ["wss://...", ...]` (default: relay.nostr.net, relay.getportal.cc).
- **Payment wallet** (optional): `[wallet]` — `ln_backend = "none" | "nwc" | "breez"`.
  - **NWC**: `[wallet.nwc]` with `url = "nostr+walletconnect://..."`.
  - **Breez**: `[wallet.breez]` with `api_key` and `mnemonic`. Data is stored in **`~/.portal-app-demo/breez`** (single session, one app instance).

Override with env: `PORTAL_APP_DEMO__<SECTION>__<KEY>=value` (double underscores).

## Run

```bash
cargo run -p portal-app-demo
```

Then open **http://127.0.0.1:3030**. If config is missing, the app creates `~/.portal-app-demo/config.toml` from the example and exits; edit identity (and optional wallet) and restart.

## Usage

1. **First run**: Edit `~/.portal-app-demo/config.toml` (identity + optional `[wallet.nwc]` or `[wallet.breez]`), then start again.
2. **UI**: Shows config path, pubkey (npub + hex), payment wallet mode, and “Waiting for payment request…”.
3. **Accept / Reject**: When a request appears, click Accept (real payment if NWC/Breez configured, else demo success) or Reject.
