# JavaScript / TypeScript SDK

The official Portal SDK for Node.js and browser apps.

**npm:** [`portal-sdk`](https://www.npmjs.com/package/portal-sdk) · **Source:** [GitHub](https://github.com/PortalTechnologiesInc/lib)

## Installation

```bash
npm install portal-sdk
```

Requires Node.js 18+ and optionally TypeScript 4.5+.

> **Compatibility:** The SDK `major.minor` version must match the SDK Daemon (`getportal/sdk-daemon`). Patch versions are independent. See [Versioning](../resources/versioning.md).

## Quick start

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'https://your-instance.hub.getportal.cc',
  authToken: 'your-auth-token',
});

// Authenticate a user
const op = await client.newKeyHandshakeUrl();
console.log('Share with user:', op.url);

const result = await client.poll(op);
console.log('User key:', result.main_key);
```

## Configuration

Choose how you want to receive async results:

```typescript
// Manual polling (default) — call poll(op) yourself
const client = new PortalClient({
  baseUrl: 'https://your-instance.hub.getportal.cc',
  authToken: 'your-auth-token',
});

// Auto-polling — background interval resolves operations automatically
const client = new PortalClient({
  baseUrl,
  authToken,
  autoPollingIntervalMs: 500,
});
// call client.destroy() to stop the scheduler when done

// Webhooks — portal-rest POSTs to your server
const client = new PortalClient({
  baseUrl,
  authToken,
  webhookSecret: 'my-secret',
});
```

| Option | Required | Description |
|--------|----------|-------------|
| `baseUrl` | Yes | HTTP base URL of your Portal instance |
| `authToken` | Yes | Bearer token matching `PORTAL__AUTH__AUTH_TOKEN` |
| `autoPollingIntervalMs` | No | Enable auto-polling; interval in ms (e.g. `500`) |
| `webhookSecret` | No | Enable webhook mode with HMAC-SHA256 verification |

## API Reference

### Auth & Users

| Method | Description |
|--------|-------------|
| `newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?)` | Get URL for user key handshake; callback runs when user completes. |
| `authenticateKey(mainKey, subkeys?)` | Authenticate a user key. Returns `AuthResponseData` with `status`, `session_token`. |

### Payments

| Method | Description |
|--------|-------------|
| `requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange)` | Request a one-time Lightning payment. |
| `requestRecurringPayment(mainKey, subkeys, paymentRequest)` | Request a recurring (subscription) payment. |
| `requestInvoicePayment(mainKey, subkeys, paymentRequest, onStatusChange)` | Pay an invoice on behalf of a user. |
| `requestInvoice(recipientKey, subkeys, content)` | Request an invoice. |
| `closeRecurringPayment(mainKey, subkeys, subscriptionId)` | Close a recurring subscription. |
| `listenClosedRecurringPayment(onClosed)` | Listen for user cancellations; returns unsubscribe function. |

### Profiles & Identity

| Method | Description |
|--------|-------------|
| `fetchProfile(mainKey)` | Fetch a user's profile (`Profile \| null`). |
| `setProfile(profile)` | Set or update a profile. |
| `fetchNip05Profile(nip05)` | Resolve a NIP-05 identifier. |

### JWT

| Method | Description |
|--------|-------------|
| `issueJwt(target_key, duration_hours)` | Issue a JWT for the given key. |
| `verifyJwt(public_key, token)` | Verify a JWT and return claims. |

### Verification

| Method | Description |
|--------|-------------|
| `createVerificationSession(relays?)` | Create an age verification session. Returns `session_url`. |
| `requestVerificationToken(recipientKey, subkeys)` | Request a verification token from a user who already holds one. |

### Cashu & Relays

| Method | Description |
|--------|-------------|
| `requestCashu(...)` | Request Cashu tokens from a user. |
| `sendCashuDirect(...)` | Send Cashu tokens to a user. |
| `mintCashu(...)` | Mint Cashu tokens. |
| `burnCashu(...)` | Burn (redeem) Cashu tokens. |
| `addRelay(relay)` | Add a relay. |
| `removeRelay(relay)` | Remove a relay. |

### Events

| Method | Description |
|--------|-------------|
| `on(eventType \| EventCallbacks, callback?)` | Register listener: `on('connected', fn)` or `on({ onConnected, onDisconnected, onError })`. |
| `off(eventType, callback)` | Remove a listener. |

## Error Handling

The SDK throws `PortalSDKError` with a `code` property:

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  const { url } = await client.newKeyHandshakeUrl();
} catch (err) {
  if (err instanceof PortalSDKError) {
    console.error(err.code, err.message);
  }
}
```

### Error codes

| Code | When |
|------|------|
| `NOT_CONNECTED` | Method called before `connect()` or after disconnect. |
| `CONNECTION_TIMEOUT` | Connection did not open in time. |
| `CONNECTION_CLOSED` | Socket closed unexpectedly. |
| `AUTH_FAILED` | Invalid or rejected auth token. |
| `UNEXPECTED_RESPONSE` | Server sent unexpected response type. |
| `SERVER_ERROR` | Server returned an error. |
| `PARSE_ERROR` | Failed to parse a message. |

## Types

- **`Currency`** — e.g. `Currency.Millisats`
- **`Timestamp`** — `Timestamp.fromDate(date)`, `Timestamp.fromNow(seconds)`, `toDate()`, `toJSON()`
- **`Profile`** — `id`, `pubkey`, `name`, `display_name`, `picture`, `about`, `nip05`
- **`SinglePaymentRequestContent`**, **`RecurringPaymentRequestContent`**, **`InvoiceRequestContent`**
- **`AuthResponseData`**, **`InvoiceStatus`**, **`RecurringPaymentStatus`**

Full types are exported from `portal-sdk`; use your editor's IntelliSense or the package source.

---

**See also:** [REST API](rest-api.md) · [Java SDK](java.md) · [OpenAPI Reference](api-reference-rest.md)
