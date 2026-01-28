# Portal TypeScript SDK

This package is the **official TypeScript/JavaScript client**: connect to a Portal endpoint to authenticate users, process payments, manage profiles, issue JWTs, and more.

## Install

```bash
npm install portal-sdk
```

## Quick start

You need two things: a **Portal endpoint** (URL) and an **auth token**. If you don’t have them yet, see [Where do I get an endpoint and token?](#where-do-i-get-an-endpoint-and-token).

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: process.env.PORTAL_URL ?? 'ws://localhost:3000/ws',
  connectTimeout: 10000,
});

await client.connect();
await client.authenticate(process.env.PORTAL_AUTH_TOKEN!);

// Generate an auth URL for a user (Nostr key handshake)
const url = await client.newKeyHandshakeUrl((mainKey, preferredRelays) => {
  console.log('User authenticated:', mainKey);
});
console.log('Share this URL with your user:', url);
```

That’s it. The SDK handles connection, protocol, and lifecycle; you call methods and react to callbacks.

---

## Where do I get an endpoint and token?

- **Hosted Portal** — Your team or provider gives you a Portal URL and token. Use them in `serverUrl` and `authenticate(token)`.
- **Run Portal yourself (Docker)** — One command, then use `ws://localhost:3000/ws` and your token:

  ```bash
  docker run -d -p 3000:3000 \
    -e AUTH_TOKEN=your-secret-token \
    -e NOSTR_KEY=your-nostr-private-key-hex \
    getportal/sdk-daemon:latest
  ```

  See [Running Portal](https://github.com/PortalTechnologies/lib#running-portal) for details.

- **Self-host / build from source** — See the main repo’s [Building from source](https://github.com/PortalTechnologies/lib/blob/main/docs/getting-started/building-from-source.md) and configuration docs.

---

## Core workflows

### Authentication (key handshake)

```typescript
const url = await client.newKeyHandshakeUrl((mainKey, preferredRelays) => {
  const auth = await client.authenticateKey(mainKey, []);
  if (auth.status.status === 'approved') {
    console.log('User approved:', mainKey);
  }
});
// Share url with user (e.g. open in browser with NWC wallet)
```

### Single payment

```typescript
import { Currency } from 'portal-sdk';

await client.requestSinglePayment(
  userPubkey,
  [],
  {
    amount: 5000,  // millisats
    currency: Currency.Millisats,
    description: 'Premium subscription',
  },
  (status) => {
    if (status.status === 'paid') console.log('Paid:', status.preimage);
    if (status.status === 'user_rejected') console.log('User declined');
  }
);
```

### Recurring payment

```typescript
import { Currency, Timestamp } from 'portal-sdk';

const result = await client.requestRecurringPayment(
  userPubkey,
  [],
  {
    amount: 10_000,
    currency: Currency.Millisats,
    recurrence: {
      calendar: 'monthly',
      first_payment_due: Timestamp.fromNow(86400),
      max_payments: 12,
    },
    expires_at: Timestamp.fromNow(3600),
  }
);
if (result.status.status === 'confirmed') {
  console.log('Subscription ID:', result.status.subscription_id);
}
```

### Fetch profile

```typescript
const profile = await client.fetchProfile(userPubkey);
if (profile) {
  console.log(profile.display_name, profile.picture);
}
```

### JWT (session tokens)

```typescript
const token = await client.issueJwt(targetPubkey, 24);  // 24 hours
const claims = await client.verifyJwt(publicKey, token);
console.log('Target key:', claims.target_key);
```

---

## Error handling

The SDK throws `PortalSDKError` with a `code` so you can handle cases in code:

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  await client.authenticate(token);
} catch (err) {
  if (err instanceof PortalSDKError) {
    switch (err.code) {
      case 'AUTH_FAILED':
        // Invalid or expired token
        break;
      case 'CONNECTION_TIMEOUT':
      case 'CONNECTION_CLOSED':
        // Connection issues
        break;
      case 'NOT_CONNECTED':
        // Call connect() first
        break;
      default:
        break;
    }
  }
  throw err;
}
```

Listen for connection/background errors:

```typescript
client.on({
  onConnected: () => console.log('Connected'),
  onDisconnected: () => console.log('Disconnected'),
  onError: (e) => console.error('Portal error:', e),
});
```

---

## Configuration

| Option | Description |
|--------|-------------|
| `serverUrl` | Portal endpoint URL (e.g. `ws://localhost:3000/ws` or your hosted URL). |
| `connectTimeout` | Connection timeout in ms. Default `10000`. |
| `debug` | Log requests/responses to console. Default `false`. |

---

## API reference

### Lifecycle

- **`connect(): Promise<void>`** — Connect to Portal. Call once before other methods.
- **`disconnect(): void`** — Close connection and clear state.
- **`authenticate(token: string): Promise<void>`** — Authenticate with your auth token (required after connect).

### Auth & users

- **`newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?): Promise<string>`** — Get URL for user key handshake; callback runs when user completes handshake.
- **`authenticateKey(mainKey, subkeys?): Promise<AuthResponseData>`** — Authenticate a user key (NIP-46 style).

### Payments

- **`requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>`**
- **`requestRecurringPayment(mainKey, subkeys, paymentRequest): Promise<RecurringPaymentResponseContent>`**
- **`requestInvoicePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>`**
- **`requestInvoice(recipientKey, subkeys, content): Promise<InvoiceResponseContent>`**
- **`closeRecurringPayment(mainKey, subkeys, subscriptionId): Promise<string>`**
- **`listenClosedRecurringPayment(onClosed): Promise<() => void>`** — Returns unsubscribe function.

### Profiles & identity

- **`fetchProfile(mainKey): Promise<Profile \| null>`**
- **`setProfile(profile): Promise<void>`**
- **`fetchNip05Profile(nip05): Promise<Nip05Profile>`**

### JWT

- **`issueJwt(target_key, duration_hours): Promise<string>`**
- **`verifyJwt(public_key, token): Promise<{ target_key: string }>`**

### Relays & Cashu

- **`addRelay(relay): Promise<string>`** / **`removeRelay(relay): Promise<string>`**
- **`requestCashu(...)`** / **`sendCashuDirect(...)`** / **`mintCashu(...)`** / **`burnCashu(...)`**
- **`calculateNextOccurrence(calendar, from): Promise<Timestamp \| null>`**

### Events

- **`on(eventType \| EventCallbacks, callback?): void`** — e.g. `on('connected', fn)` or `on({ onConnected, onDisconnected, onError })`.
- **`off(eventType, callback): void`** — Remove listener.

---

## Types (overview)

- **`Currency`** — `Currency.Millisats`.
- **`Timestamp`** — `Timestamp.fromDate(date)`, `Timestamp.fromNow(seconds)`, `toDate()`, `toJSON()`.
- **`Profile`** — `id`, `pubkey`, `name`, `display_name`, `picture`, `about`, `nip05`.
- **`RecurringPaymentRequestContent`** / **`SinglePaymentRequestContent`** / **`InvoiceRequestContent`** — See TypeScript definitions.
- **`AuthResponseData`**, **`InvoiceStatus`**, **`RecurringPaymentStatus`** — Response and status types.

Full types are exported from the package; use your editor’s IntelliSense or the source.

---

## Examples (snippets)

**Auth flow + cleanup:**

```typescript
const client = new PortalSDK({ serverUrl: process.env.PORTAL_URL! });
try {
  await client.connect();
  await client.authenticate(process.env.PORTAL_AUTH_TOKEN!);
  const url = await client.newKeyHandshakeUrl((key) => console.log('User:', key));
  console.log(url);
} finally {
  client.disconnect();
}
```

**Payment with status:**

```typescript
await client.requestSinglePayment(
  pubkey, [], { amount: 1000, currency: Currency.Millisats, description: 'Tip' },
  (status) => console.log(status.status)
);
```

---

## Advanced

- **Running Portal yourself** — Docker, env vars, and building from source are documented in the [main repo](https://github.com/PortalTechnologies/lib) and [docs](https://github.com/PortalTechnologies/lib/tree/main/docs).
- **Browser** — The SDK works in Node and browser; WebSocket is handled by the bundled client.
- **Other languages** — [Java SDK](https://github.com/PortalTechnologiesInc/jvm-client). For raw API access, see the portal-rest [API reference](https://github.com/PortalTechnologies/lib/tree/main/crates/portal-rest#api-endpoints) (advanced).

---

## License

MIT — see [LICENSE](../../LICENSE) in the repo.
