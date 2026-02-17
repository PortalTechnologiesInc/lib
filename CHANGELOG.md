# Changelog

All notable changes to Portal libraries will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## portal-rest (SDK Daemon)

### Unreleased

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
