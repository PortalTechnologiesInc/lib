# Portal TypeScript SDK

Official **TypeScript/JavaScript client** for the Portal REST API: authenticate users, process payments, manage profiles, issue JWTs, and more.

**Full documentation:** [https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/)

---

## Install

```bash
npm install portal-sdk
```

## Requirements

- Node.js 18+ (uses native `fetch`)
- Works in modern browsers (no polyfills needed)
- No WebSocket dependency — pure REST + polling

## Quick start

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  baseUrl: process.env.PORTAL_URL ?? 'http://localhost:3000',
  authToken: process.env.PORTAL_AUTH_TOKEN!,
});

// Key handshake flow
const { url, stream_id } = await client.newKeyHandshakeUrl();
console.log('Share this URL with your user:', url);

// Poll until handshake completes
const result = await client.pollUntilDone(stream_id, (event) => {
  console.log('Event:', event.type);
});
console.log('User authenticated:', result);
```

## API Overview

### Configuration

```typescript
const client = new PortalSDK({
  baseUrl: 'http://localhost:3000', // Portal REST API base URL
  authToken: 'your-bearer-token',  // Optional, can set later with setAuthToken()
  debug: false,                     // Enable console debug logging
});
```

### Async Operations & Polling

Some operations (key handshake, payments) are asynchronous. They return a `stream_id` immediately. Use `pollUntilDone()` to wait for completion:

```typescript
// Start a payment
const { stream_id } = await client.requestSinglePayment(mainKey, [], {
  amount: 1000,
  currency: 'Millisats',
  description: 'Test payment',
});

// Poll for status updates
const finalEvent = await client.pollUntilDone(stream_id, (event) => {
  console.log('Status update:', event);
}, {
  intervalMs: 1000,  // Poll every 1s (default)
  timeoutMs: 300000, // 5 minute timeout (optional)
});
```

### Available Methods

| Method | Description |
|--------|-------------|
| `health()` | Health check |
| `version()` | Server version info |
| `newKeyHandshakeUrl()` | Create key handshake URL (returns stream_id) |
| `authenticateKey()` | Authenticate a key (NIP-46) |
| `requestSinglePayment()` | Request single payment (returns stream_id) |
| `requestPaymentRaw()` | Request payment with raw content (returns stream_id) |
| `requestRecurringPayment()` | Request recurring payment |
| `closeRecurringPayment()` | Close a recurring payment |
| `listenClosedRecurringPayment()` | Listen for closed recurring payments (returns stream_id) |
| `fetchProfile()` | Fetch user profile |
| `setProfile()` | Set user profile |
| `requestInvoice()` | Request invoice from recipient |
| `payInvoice()` | Pay a BOLT11 invoice |
| `issueJwt()` | Issue a JWT |
| `verifyJwt()` | Verify a JWT |
| `requestCashu()` | Request Cashu tokens |
| `sendCashuDirect()` | Send Cashu tokens directly |
| `mintCashu()` | Mint Cashu tokens |
| `burnCashu()` | Burn (receive) Cashu tokens |
| `addRelay()` | Add a relay |
| `removeRelay()` | Remove a relay |
| `calculateNextOccurrence()` | Calculate next calendar occurrence |
| `fetchNip05Profile()` | Fetch NIP-05 profile |
| `getWalletInfo()` | Get wallet info |
| `getEvents()` | Fetch events for a stream |
| `pollUntilDone()` | Poll until terminal event |

### Webhooks

The server can be configured with a `webhook_url` to POST events as they happen. The SDK exports `WebhookPayload` type for typing your webhook handler:

```typescript
import { WebhookPayload } from 'portal-sdk';
import crypto from 'crypto';

// In your webhook handler (Express, etc.)
app.post('/webhook', (req, res) => {
  const payload: WebhookPayload = req.body;
  
  // Verify HMAC signature if webhook_secret is configured
  const signature = req.headers['x-portal-signature'] as string;
  const expected = crypto
    .createHmac('sha256', process.env.WEBHOOK_SECRET!)
    .update(JSON.stringify(req.body))
    .digest('hex');
  
  if (signature !== expected) {
    return res.status(401).send('Invalid signature');
  }

  console.log('Webhook event:', payload.type, 'Stream:', payload.stream_id);
  res.sendStatus(200);
});
```

### Error Handling

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  await client.requestSinglePayment(/* ... */);
} catch (err) {
  if (err instanceof PortalSDKError) {
    console.error('Code:', err.code);       // 'API_ERROR', 'HTTP_ERROR', etc.
    console.error('Status:', err.statusCode); // HTTP status code
    console.error('Message:', err.message);
  }
}
```

Error codes: `HTTP_ERROR`, `API_ERROR`, `POLL_TIMEOUT`, `PARSE_ERROR`, `NETWORK_ERROR`.

---

## License

MIT — see [LICENSE](../../LICENSE) in the repo.
