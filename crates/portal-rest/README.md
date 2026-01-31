# Portal API

This crate is the **Portal API** (REST + WebSocket): the server that exposes that functionality. Use it via the **official SDKs** (TypeScript, Java) or connect to the WebSocket API directly.

## Use the SDK

Install the SDK for your language and connect to a Portal endpoint with an auth token.

### TypeScript / JavaScript

```bash
npm install portal-sdk
```

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: process.env.PORTAL_URL ?? 'ws://localhost:3000/ws',
});

await client.connect();
await client.authenticate(process.env.PORTAL_AUTH_TOKEN!);

// e.g. generate auth URL for a user
const url = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('User authenticated:', mainKey);
});
```

**Full SDK docs:** [TypeScript SDK](clients/ts/README.md) — quick start, workflows, API reference, error handling.

### Java

[Java client](https://github.com/PortalTechnologiesInc/jvm-client) — use the same pattern: connect to a Portal endpoint and authenticate with a token.

---

## Run the API

For self-hosting or local development: run the Portal API (Docker or from source). The SDK connects to it via the API URL and your auth token.

### Quick run with Docker

```bash
docker run -d -p 3000:3000 \
  -e AUTH_TOKEN=your-secret-token \
  -e NOSTR_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Then use `ws://localhost:3000/ws` as `serverUrl` and `your-secret-token` in `client.authenticate(...)`.

**Verify:** `curl http://localhost:3000/health` → `OK`

### Configuration (file / env)

Portal creates `~/.portal-rest` and a default config if missing. You can customize:

- **Config file:** `~/.portal-rest/config.toml` (copy from `example.config.toml` in this crate).
- **Environment:** `PORTAL__<SECTION>__<KEY>=value` (e.g. `PORTAL__AUTH__AUTH_TOKEN=...`, `PORTAL__WALLET__LN_BACKEND=nwc`).

| Config key | Env example | Description |
|------------|-------------|-------------|
| `info.listen_port` | `PORTAL__INFO__LISTEN_PORT` | Port (default 3000) |
| `auth.auth_token` | `PORTAL__AUTH__AUTH_TOKEN` | API auth token |
| `nostr.private_key` | `PORTAL__NOSTR__PRIVATE_KEY` | Nostr key (hex) |
| `nostr.relays` | `PORTAL__NOSTR__RELAYS` | Relay URLs (comma-separated) |
| `wallet.ln_backend` | `PORTAL__WALLET__LN_BACKEND` | `none`, `nwc`, or `breez` |
| `wallet.nwc.url` | `PORTAL__WALLET__NWC__URL` | Nostr Wallet Connect URL |

**Run from config:**

```bash
portal-rest   # uses ~/.portal-rest/config.toml
```

### Build from source

```bash
cargo build --release
./target/release/portal-rest   # or binary name from this crate
```

Server listens on `127.0.0.1:3000` by default.

### Docker image (Nix build)

```bash
nix build .#rest-docker
docker load < result
```

Multi-arch (amd64/arm64): build on each arch, tag and push, then create a manifest; see repo CI or docs for exact steps.

---

## API reference (advanced)

If you’re **not** using an SDK (e.g. another language or custom client), you can talk to Portal over the WebSocket API.

- **Endpoint:** `GET /ws` (WebSocket).
- **Auth:** First message must be an `Auth` command with your token.
- **Protocol:** JSON commands with an `id`, `cmd`, and optional `params`; responses match on `id`.

### Example commands (overview)

| Command | Purpose |
|---------|--------|
| `Auth` | Authenticate with token (required first). |
| `NewKeyHandshakeUrl` | Get URL for user key handshake. |
| `AuthenticateKey` | Authenticate a user key. |
| `RequestSinglePayment` / `RequestRecurringPayment` | Payments. |
| `FetchProfile` / `SetProfile` | Profiles. |
| `IssueJwt` / `VerifyJwt` | JWT. |
| `AddRelay` / `RemoveRelay` | Relays. |

**REST:** `GET /health` → `OK` (health check).

Full request/response shapes and more commands are in the [WebSocket API section](#api-endpoints) below. For day-to-day use, the [TypeScript SDK](clients/ts/README.md) (and Java client) hide these details.

---

## API Endpoints

### Authentication

All API usage is authenticated. With the WebSocket API, the first command must be:

```json
{ "id": "<unique-id>", "cmd": "Auth", "params": { "token": "<AUTH_TOKEN>" } }
```

### REST

- `GET /health` — Health check, returns `OK`.
- `GET /ws` — WebSocket upgrade for real-time API.

### WebSocket protocol

- Send JSON messages: `{ "id": "...", "cmd": "<Command>", "params": { ... } }`.
- Receive JSON: `{ "type": "success", "id": "...", "data": ... }` or `{ "type": "error", "id": "...", "message": "..." }`.
- Notifications (e.g. payment status) use `type: "notification"` and the same `id` as the stream.

### Available commands (reference)

#### `Auth`

```json
{ "id": "unique-id", "cmd": "Auth", "params": { "token": "<AUTH_TOKEN>" } }
```

#### `NewKeyHandshakeUrl`

```json
{ "id": "unique-id", "cmd": "NewKeyHandshakeUrl" }
```

#### `AuthenticateKey`

```json
{
  "id": "unique-id",
  "cmd": "AuthenticateKey",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": ["hex_encoded_pub_key", ...]
  }
}
```

#### `RequestRecurringPayment`

```json
{
  "id": "unique-id",
  "cmd": "RequestRecurringPayment",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": [],
    "payment_request": { /* RecurringPaymentRequestContent */ }
  }
}
```

#### `RequestSinglePayment`

```json
{
  "id": "unique-id",
  "cmd": "RequestSinglePayment",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": [],
    "payment_request": { /* SinglePaymentRequestContent */ }
  }
}
```

#### `FetchProfile`

```json
{ "id": "unique-id", "cmd": "FetchProfile", "params": { "main_key": "hex_encoded_pub_key" } }
```

#### `IssueJwt`

```json
{
  "id": "unique-id",
  "cmd": "IssueJwt",
  "params": { "target_key": "hex_encoded_pub_key", "duration_hours": 24 }
}
```

#### `VerifyJwt`

```json
{ "id": "unique-id", "cmd": "VerifyJwt", "params": { "pubkey": "...", "token": "..." } }
```

Other commands (`SetProfile`, `CloseRecurringPayment`, `RequestInvoice`, Cashu, relays, etc.) follow the same pattern; see TypeScript SDK types or server source for full shapes.

### Raw WebSocket example (no SDK)

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({ id: '1', cmd: 'Auth', params: { token: 'your-auth-token' } }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'success' && msg.data?.type === 'auth_success') {
    // Authenticated; send other commands with new ids
  }
};
```

For production apps, use the [TypeScript SDK](clients/ts/README.md) or [Java client](https://github.com/PortalTechnologiesInc/jvm-client) instead of raw WebSocket.
