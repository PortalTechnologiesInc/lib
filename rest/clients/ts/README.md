# Portal SDK - TypeScript Client

A TypeScript client for the Portal WebSocket Server, providing Nostr-based authentication and Lightning Network payment processing capabilities.

## Installation

```bash
npm install portal-sdk
```

## Quick Start

```typescript
import { PortalSDK, Currency, Timestamp } from 'portal-sdk';

// Initialize the client
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  connectTimeout: 10000
});

// Connect to the server
await client.connect();

// Authenticate with your token
await client.authenticate('your-auth-token');

// Generate authentication URL for users
const url = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('Received key handshake from:', mainKey);
});

console.log('Authentication URL:', url);
```

## Features

- **Nostr Authentication**: Secure user authentication using Nostr protocol
- **Lightning Payments**: Single and recurring payment processing
- **Profile Management**: Fetch and update user profiles
- **JWT Support**: Issue and verify JWT tokens
- **Relay Management**: Add and remove Nostr relays dynamically
- **Real-time Updates**: WebSocket-based real-time notifications
- **TypeScript Support**: Full TypeScript definitions included

## API Reference

### PortalSDK Class

The main client class for interacting with the Portal server.

#### Constructor

```typescript
new PortalSDK(config: ClientConfig)
```

**Parameters:**
- `config.serverUrl` (string): WebSocket server URL
- `config.connectTimeout` (number, optional): Connection timeout in milliseconds (default: 10000)

#### Methods

##### `connect(): Promise<void>`

Establishes a WebSocket connection to the server.

```typescript
await client.connect();
```

##### `disconnect(): void`

Closes the WebSocket connection and cleans up resources.

```typescript
client.disconnect();
```

##### `authenticate(token: string): Promise<void>`

Authenticates with the server using a token.

```typescript
await client.authenticate('your-auth-token');
```

##### `newKeyHandshakeUrl(onKeyHandshake: (mainKey: string, preferredRelays: string[]) => void, staticToken?: string): Promise<string>`

Generates a new authentication URL for user key handshake.

```typescript
const url = await client.newKeyHandshakeUrl((mainKey, preferredRelays) => {
  console.log('Recevied key handshake from:', mainKey);
  console.log('User wants to talk at:', preferredRelays);
});
```

##### `authenticateKey(mainKey: string, subkeys?: string[]): Promise<AuthResponseData>`

Authenticates a user's key with optional subkeys.

```typescript
const authResponse = await client.authenticateKey('user-pubkey', ['subkey1', 'subkey2']);
```

##### `requestRecurringPayment(mainKey: string, subkeys: string[], paymentRequest: RecurringPaymentRequestContent): Promise<RecurringPaymentResponseContent>`

Requests a recurring payment subscription.

```typescript
const paymentRequest: RecurringPaymentRequestContent = {
  amount: 10000, // 10 sats
  currency: Currency.Millisats,
  recurrence: {
    calendar: "monthly",
    first_payment_due: Timestamp.fromNow(86400), // 24 hours from now
    max_payments: 12
  },
  expires_at: Timestamp.fromNow(3600) // 1 hour from now
};

const result = await client.requestRecurringPayment('user-pubkey', [], paymentRequest);
```

##### `requestSinglePayment(mainKey: string, subkeys: string[], paymentRequest: SinglePaymentRequestContent, onStatusChange: (status: InvoiceStatus) => void): Promise<void>`

Requests a single payment with the NWC embedded wallet.

```typescript
const paymentRequest: SinglePaymentRequestContent = {
  amount: 5000, // 5 sats
  currency: Currency.Millisats,
  description: "Product purchase"
};

await client.requestSinglePayment('user-pubkey', [], paymentRequest, (status) => {
  console.log('Payment status:', status);
});
```

##### `requestInvoicePayment(mainKey: string, subkeys: string[], paymentRequest: InvoicePaymentRequestContent, onStatusChange: (status: InvoiceStatus) => void): Promise<void>`

Requests payment for a specific invoice (generated outside of the NWC wallet).

```typescript
const paymentRequest: InvoicePaymentRequestContent = {
  amount: 1000,
  currency: Currency.Millisats,
  description: "Invoice payment",
  invoice: "lnbc..." // Lightning invoice
};

await client.requestInvoicePayment('user-pubkey', [], paymentRequest, (status) => {
  console.log('Invoice payment status:', status);
});
```

##### `fetchProfile(mainKey: string): Promise<Profile | null>`

Fetches a user's profile.

```typescript
const profile = await client.fetchProfile('user-pubkey');
console.log('User profile:', profile);
```

##### `setProfile(profile: Profile): Promise<void>`

Updates the service profile.

```typescript
const profile: Profile = {
  id: 'user-id',
  pubkey: 'user-pubkey',
  name: 'John Doe',
  display_name: 'John',
  picture: 'https://example.com/avatar.jpg',
  about: 'Software developer',
  nip05: 'john@example.com' // Read NIP-05 for the spec of /.well-known/nostr.json
};

await client.setProfile(profile);
```

##### `closeRecurringPayment(mainKey: string, subkeys: string[], subscriptionId: string): Promise<string>`

Closes a recurring payment subscription.

```typescript
const message = await client.closeRecurringPayment('user-pubkey', [], 'subscription-id');
console.log('Subscription closed:', message);
```

##### `listenClosedRecurringPayment(onClosed: (data: CloseRecurringPaymentNotification) => void): Promise<void>`

Listens for closed recurring payment notifications.

```typescript
await client.listenClosedRecurringPayment((data) => {
  console.log('Payment closed:', data);
});
```

##### `requestInvoice(recipientKey: string, content: InvoicePaymentRequestContent): Promise<InvoiceResponseContent>`

Requests an invoice for payment.

```typescript
const content: InvoicePaymentRequestContent = {
  amount: 1000,
  currency: Currency.Millisats,
  description: "Invoice request"
};

const invoice = await client.requestInvoice('recipient-pubkey', content);
console.log('Invoice:', invoice.invoice);
```

##### `issueJwt(targetKey: string, durationHours: number): Promise<string>`

Issues a JWT token for a target key.

```typescript
const token = await client.issueJwt('target-pubkey', 24); // 24 hours
console.log('JWT token:', token);
```

##### `verifyJwt(publicKey: string, token: string): Promise<{ targetKey: string }>`

Verifies a JWT token.

```typescript
const claims = await client.verifyJwt('public-key', 'jwt-token');
console.log('Token target key:', claims.targetKey);
```

##### `addRelay(relay: string): Promise<string>`

Adds a relay to the relay pool.

```typescript
const relayUrl = await client.addRelay('wss://relay.damus.io');
console.log('Added relay:', relayUrl);
```

##### `removeRelay(relay: string): Promise<string>`

Removes a relay from the relay pool.

```typescript
const relayUrl = await client.removeRelay('wss://relay.damus.io');
console.log('Removed relay:', relayUrl);
```

##### `on(eventType: string | EventCallbacks, callback?: (data: any) => void): void`

Registers event listeners.

```typescript
// Single event
client.on('connected', () => {
  console.log('Connected to server');
});

// Multiple events
client.on({
  onConnected: () => console.log('Connected'),
  onDisconnected: () => console.log('Disconnected'),
  onError: (error) => console.error('Error:', error)
});
```

##### `off(eventType: string, callback: (data: any) => void): void`

Removes an event listener.

```typescript
const callback = (data) => console.log(data);
client.on('event', callback);
client.off('event', callback);
```

## Types

### Currency

```typescript
enum Currency {
  Millisats = "Millisats"
}
```

### Timestamp

```typescript
class Timestamp {
  static fromDate(date: Date): Timestamp
  static fromNow(seconds: number): Timestamp
  toJSON(): string
  toString(): string
  valueOf(): bigint
}
```

### Profile

```typescript
interface Profile {
  id: string;
  pubkey: string;
  name?: string;
  display_name?: string;
  picture?: string;
  about?: string;
  nip05?: string;
}
```

### Payment Types

```typescript
interface RecurringPaymentRequestContent {
  amount: number;
  currency: Currency;
  recurrence: RecurrenceInfo;
  current_exchange_rate?: any;
  expires_at: Timestamp;
  auth_token?: string;
}

interface SinglePaymentRequestContent {
  description: string;
  amount: number;
  currency: Currency;
  subscription_id?: string;
  auth_token?: string;
}

interface InvoicePaymentRequestContent {
  amount: number;
  currency: Currency;
  description: string;
  subscription_id?: string;
  auth_token?: string;
  current_exchange_rate?: any;
  expires_at?: Timestamp;
  invoice?: string;
}
```

### Response Types

```typescript
interface AuthResponseData {
  user_key: string;
  recipient: string;
  challenge: string;
  status: AuthResponseStatus;
}

interface InvoiceStatus {
  status: 'paid' | 'timeout' | 'error' | 'user_approved' | 'user_success' | 'user_failed' | 'user_rejected';
  preimage?: string;
  reason?: string;
}
```

## Examples

### Complete Authentication Flow

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

try {
  await client.connect();
  await client.authenticate('your-auth-token');
  
  const url = await client.newKeyHandshakeUrl((mainKey) => {
    console.log('Received key handshake from:', mainKey);
    const authResponse = await client.authenticateKey(mainKey, []);
    console.log('Auth response:', authResponse);
  });
  
  console.log('Share this URL with the user:', url);
} catch (error) {
  console.error('Authentication failed:', error);
} finally {
  client.disconnect();
}
```

### Payment Processing

```typescript
import { PortalSDK, Currency, Timestamp } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate('your-auth-token');

// Request a single payment
await client.requestSinglePayment(
  'user-pubkey',
  [],
  {
    amount: 1000,
    currency: Currency.Millisats,
    description: "Product purchase"
  },
  (status) => {
    if (status.status === 'paid') {
      console.log('Payment completed!');
    } else if (status.status === 'timeout') {
      console.log('Payment timed out');
    }
  }
);
```

### Profile Management

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate('your-auth-token');

// Fetch user profile
const profile = await client.fetchProfile('user-pubkey');
console.log('User profile:', profile);
```

### Relay Management

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate('your-auth-token');

// Add a relay to the relay pool
const addedRelay = await client.addRelay('wss://relay.damus.io');
console.log('Added relay:', addedRelay);

// Remove a relay from the relay pool
const removedRelay = await client.removeRelay('wss://relay.damus.io');
console.log('Removed relay:', removedRelay);
```

## Error Handling

The client throws errors for various scenarios:

```typescript
try {
  await client.connect();
  await client.authenticate('invalid-token');
} catch (error) {
  if (error.message.includes('Authentication failed')) {
    console.error('Invalid authentication token');
  } else if (error.message.includes('Connection timeout')) {
    console.error('Server connection timeout');
  }
}
```

## Browser Support

This client works in both Node.js and browser environments. For browser usage, the WebSocket implementation is automatically handled by the `isomorphic-ws` package.

## License

MIT License - see LICENSE file for details. 