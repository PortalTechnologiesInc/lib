# Portal TypeScript SDK

Server-side TypeScript/Node.js client for the [Portal REST API](https://github.com/PortalTechnologiesInc/lib).

## Install

```bash
npm install portal-sdk
```

**Requirements:** Node.js 18+ (server-side only — uses native `fetch` and `crypto`)

## Setup

Choose how you want to receive async results:

```ts
// Manual polling — call poll(op) yourself, no background timers (default)
const client = new PortalClient({ baseUrl: 'http://localhost:3000', authToken });

// Auto-polling — background interval resolves done automatically
const client = new PortalClient({ baseUrl, authToken, autoPollingIntervalMs: 500 });
// call client.destroy() to stop the scheduler when done

// Webhooks — portal-rest POSTs to your server
const client = new PortalClient({ baseUrl, authToken, webhookSecret: 'my-secret' });
```

## Async operations

All async methods return `AsyncOperation<T>` immediately:
- `streamId` — available right away
- `done` — `Promise<T>` that resolves when the operation completes

### Manual polling

```ts
const op = await client.requestSinglePayment(mainKey, [], {
  description: 'Coffee',
  amount: 1000,
  currency: Currency.Millisats,
});

const result = await client.poll(op, { timeoutMs: 60_000 });
console.log(result.status); // "paid", "timeout", "user_rejected", ...
```

### Auto-polling

```ts
// client configured with autoPollingIntervalMs: 500
const op = await client.requestSinglePayment(...);
const result = await op.done;
console.log(result.status);
```

### Webhooks

```ts
// client configured with webhookSecret
const op = await client.requestSinglePayment(...);
op.done.then(result => console.log(result.status));

// mount once in your HTTP server — do NOT parse body before this route
app.post('/portal/webhook', client.webhookHandler());
```

With raw Node.js HTTP:

```ts
const server = createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/portal/webhook') {
    return client.webhookHandler()(req, res);
  }
});
```

## Async methods

| Method | Resolves to |
|--------|-------------|
| `requestSinglePayment(mainKey, subkeys, content)` | `AsyncOperation<InvoiceStatus>` |
| `requestPaymentRaw(mainKey, subkeys, content)` | `AsyncOperation<InvoiceStatus>` |
| `requestRecurringPayment(mainKey, subkeys, content)` | `AsyncOperation<RecurringPaymentResponseContent>` |
| `requestInvoice(recipientKey, subkeys, params)` | `AsyncOperation<InvoicePaymentResponse>` |
| `requestCashu(recipientKey, subkeys, mintUrl, unit, amount)` | `AsyncOperation<CashuResponseStatus>` |
| `authenticateKey(mainKey, subkeys)` | `AsyncOperation<AuthResponseData>` |
| `newKeyHandshakeUrl(opts?)` | `AsyncOperation<KeyHandshakeResult>` |

## Sync methods

`health()`, `version()`, `info()`, `fetchProfile()`, `payInvoice()`,
`closeRecurringPayment()`, `issueJwt()`, `verifyJwt()`, `addRelay()`, `removeRelay()`,
`mintCashu()`, `burnCashu()`, `sendCashuDirect()`, `calculateNextOccurrence()`,
`fetchNip05Profile()`, `getWalletInfo()`

## Intermediate events

Subscribe to non-terminal events on a stream (e.g. `user_approved` before `paid`):

```ts
const unsub = client.onEvent(op.streamId, (event) => {
  console.log(event.type);
});
// call unsub() to unsubscribe
```

## Error handling

```ts
import { PortalSDKError } from 'portal-sdk';

try {
  const op = await client.requestSinglePayment(...);
  const result = await client.poll(op);
} catch (err) {
  if (err instanceof PortalSDKError) {
    console.error(err.code);    // 'API_ERROR', 'POLL_TIMEOUT', 'SIGNATURE_INVALID', ...
    console.error(err.message);
  }
}
```

## Versioning

SDK `major.minor` must match the portal-rest (sdk-daemon) version.

| portal-sdk | sdk-daemon |
|------------|------------|
| 0.4.x      | 0.4.x      |
| 0.3.x      | 0.3.x      |
