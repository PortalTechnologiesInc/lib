# Portal TypeScript SDK

Official **server-side TypeScript/Node.js client** for the Portal REST API: authenticate users, process payments, manage profiles, issue JWTs, and more.

**Full documentation:** [https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/)

---

## Install

```bash
npm install portal-sdk
```

## Requirements

- **Node.js 18+** (server-side only — uses native `fetch` and `crypto`)
- No browser support (this is a server SDK)

## Quick start

```typescript
import { createServer } from 'http';
import { PortalClient } from 'portal-sdk';

const portal = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: process.env.PORTAL_AUTH_TOKEN!,
  webhookSecret: process.env.PORTAL_WEBHOOK_SECRET!,
});

// Mount the webhook handler in your HTTP server
const server = createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/portal/webhook') {
    return portal.webhookHandler()(req, res);
  }
  res.writeHead(404);
  res.end();
});
server.listen(4000);

// Key handshake — resolved via webhook
const handshake = await portal.newKeyHandshakeUrl();
console.log('URL:', handshake.url);

const { main_key } = await handshake.done; // ← resolves when webhook fires
console.log('User authenticated:', main_key);

// Payment — resolved via webhook
const payment = await portal.requestSinglePayment(main_key, [], {
  amount: 1000,
  currency: 'Millisats',
  description: 'Test',
});

// Track intermediate status changes
portal.onEvent(payment.streamId, (event) => {
  console.log('Status:', event);
});

const result = await payment.done; // ← resolves on terminal event
console.log('Payment result:', result);
```

### With Express

```typescript
import express from 'express';
import { PortalClient } from 'portal-sdk';

const app = express();
const portal = new PortalClient({ baseUrl, authToken, webhookSecret });

// Important: do NOT use express.json() on the webhook route.
// The handler reads the raw body for signature verification.
app.post('/portal/webhook', portal.webhookHandler());

// Parse JSON for other routes
app.use(express.json());

app.post('/pay', async (req, res) => {
  const payment = await portal.requestSinglePayment(/* ... */);
  const result = await payment.done;
  res.json(result);
});

app.listen(4000);
```

## Architecture

### Webhook-first async operations

Async operations (key handshake, payments) return an `AsyncOperation`:

```typescript
interface AsyncOperation<T> {
  streamId: string;   // Available immediately
  done: Promise<T>;   // Resolves when terminal webhook fires
}
```

The `done` promise is resolved by the webhook handler when portal-rest POSTs the terminal event.

### Event listeners

Use `onEvent()` for intermediate events (e.g. `user_approved` before `paid`):

```typescript
portal.onEvent(streamId, (event) => {
  console.log(event.type, event);
});
```

Returns an unsubscribe function.

### Polling fallback

If webhooks aren't available, use `poll()`:

```typescript
const result = await portal.poll(streamId, {
  intervalMs: 1000,
  timeoutMs: 60000,
  onEvent: (event) => console.log(event),
});
```

### Cleanup

Cancel a pending stream (rejects its `done` promise):

```typescript
portal.destroy(streamId, 'Cancelled by user');
```

## Webhook signature verification

The SDK verifies signatures automatically in `webhookHandler()`. For manual verification:

```typescript
import { verifyWebhookSignature, constructWebhookEvent } from 'portal-sdk';

// Verify only
verifyWebhookSignature(rawBody, signature, secret); // throws on failure

// Verify + parse
const event = constructWebhookEvent(rawBody, signature, secret);
```

## API Reference

### Async operations (return `AsyncOperation`)

| Method | Description |
|--------|-------------|
| `newKeyHandshakeUrl()` | Create key handshake URL |
| `requestSinglePayment()` | Request single payment |
| `requestPaymentRaw()` | Request payment with raw content |

### Synchronous operations

| Method | Description |
|--------|-------------|
| `health()` | Health check |
| `version()` | Server version |
| `authenticateKey()` | Authenticate a key (NIP-46) |
| `requestRecurringPayment()` | Request recurring payment |
| `closeRecurringPayment()` | Close recurring payment |
| `listenClosedRecurringPayment()` | Listen for closed recurring payments |
| `fetchProfile()` / `setProfile()` | Profile management |
| `requestInvoice()` | Request invoice from recipient |
| `payInvoice()` | Pay BOLT11 invoice |
| `issueJwt()` / `verifyJwt()` | JWT operations |
| `requestCashu()` / `sendCashuDirect()` | Cashu token operations |
| `mintCashu()` / `burnCashu()` | Cashu mint/burn |
| `addRelay()` / `removeRelay()` | Relay management |
| `calculateNextOccurrence()` | Calendar operations |
| `fetchNip05Profile()` | NIP-05 lookup |
| `getWalletInfo()` | Wallet info |
| `getEvents()` | Low-level event fetch |

### Stream management

| Method | Description |
|--------|-------------|
| `webhookHandler()` | Returns HTTP handler for webhook route |
| `onEvent(streamId, cb)` | Subscribe to all events on a stream |
| `poll(streamId, opts)` | Polling fallback |
| `destroy(streamId)` | Cancel a pending stream |
| `pendingCount` | Number of active pending streams |

## Error handling

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  await portal.requestSinglePayment(/* ... */);
} catch (err) {
  if (err instanceof PortalSDKError) {
    console.error(err.code);       // 'API_ERROR', 'HTTP_ERROR', etc.
    console.error(err.statusCode);  // HTTP status code
    console.error(err.message);
  }
}
```

Error codes: `HTTP_ERROR`, `API_ERROR`, `POLL_TIMEOUT`, `PARSE_ERROR`, `NETWORK_ERROR`, `SIGNATURE_INVALID`.

---

## License

MIT — see [LICENSE](../../LICENSE) in the repo.
