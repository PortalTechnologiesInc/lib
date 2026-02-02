# Basic Usage

Learn the fundamentals of using the Portal SDK.

## Quick Example

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

Here's a complete example showing the basic workflow:

```typescript
import { PortalSDK } from 'portal-sdk';

async function main() {
  // 1. Create client
  const client = new PortalSDK({
    serverUrl: 'ws://localhost:3000/ws'
  });

  // 2. Connect
  await client.connect();
  console.log('✅ Connected');

  // 3. Authenticate
  await client.authenticate('your-auth-token');
  console.log('✅ Authenticated');

  // 4. Generate auth URL for user
  const authUrl = await client.newKeyHandshakeUrl((mainKey) => {
    console.log('✅ User authenticated:', mainKey);
  });
  
  console.log('Share this URL:', authUrl);

  // Keep running...
  await new Promise(() => {});
}

main().catch(console.error);
```

</section>

<div slot="title">Java</div>
<section>

```java
var portalSDK = new PortalSDK(healthEndpoint, wsEndpoint);
portalSDK.connect(authToken);

portalSDK.sendCommand(new CalculateNextOccurrenceRequest("weekly", System.currentTimeMillis() / 1000), (res, err) -> {
    if (err != null) {
        logger.error("error: {}", err);
        return;
    }
    logger.info("next occurrence: {}", res.next_occurrence());
});
```

</section>

</custom-tabs>

## Core Concepts

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### 1. Client Initialization

Create a Portal client instance:

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  connectTimeout: 10000  // Optional: timeout in milliseconds
});
```

**Configuration Options:**
- `serverUrl` (required): Portal endpoint URL (e.g. `ws://localhost:3000/ws` or your hosted URL)
- `connectTimeout` (optional): Connection timeout in ms (default: 10000)

### 2. Connection Management

#### Connect

```typescript
try {
  await client.connect();
  console.log('Connected successfully');
} catch (error) {
  console.error('Connection failed:', error);
}
```

#### Disconnect

```typescript
client.disconnect();
console.log('Disconnected');
```

#### Connection Events

```typescript
client.on({
  onConnected: () => {
    console.log('Connection established');
  },
  onDisconnected: () => {
    console.log('Connection closed');
  },
  onError: (error) => {
    console.error('Connection error:', error);
  }
});
```

### 3. Authentication

Authenticate with Portal using your auth token:

```typescript
try {
  await client.authenticate('your-auth-token');
  console.log('Authenticated with Portal');
} catch (error) {
  console.error('Authentication failed:', error);
}
```

**Important**: You must authenticate before calling any other API methods.

</section>

<div slot="title">Java</div>
<section>

### 1. Client Initialization

```java
var portalSDK = new PortalSDK(healthEndpoint, wsEndpoint);
```

### 2. Connection and authentication

```java
portalSDK.connect(authToken);
```

`connect` establishes the WebSocket and authenticates with the given token.

### 3. Sending commands

Use `sendCommand(request, callback)` for all server commands. Key request classes include **AuthRequest**, **KeyHandshakeUrlRequest**, **RequestSinglePaymentRequest**, **MintCashuRequest**; see the [API Reference](api-reference.md) for the full list.

**Important**: You must call `connect(authToken)` before sending commands.

</section>

</custom-tabs>

## Basic Workflows

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### User Authentication Flow

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

// Generate authentication URL
const authUrl = await client.newKeyHandshakeUrl(async (userPubkey, preferredRelays) => {
  console.log('User pubkey:', userPubkey);
  console.log('User relays:', preferredRelays);
  
  // Authenticate the user's key
  const authResponse = await client.authenticateKey(userPubkey);
  
  if (authResponse.status.status === 'approved') {
    console.log('User approved authentication!');
    // Create session, store user info, etc.
  } else {
    console.log('User declined authentication');
  }
});

console.log('Send this to user:', authUrl);
```

### Request a Single Payment

```typescript
import { PortalSDK, Currency } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

const userPubkey = 'user-public-key-hex';

await client.requestSinglePayment(
  userPubkey,
  [], // subkeys (optional)
  {
    amount: 5000, // 5 sats (in millisats)
    currency: Currency.Millisats,
    description: 'Premium subscription'
  },
  (status) => {
    console.log('Payment status:', status.status);
    
    if (status.status === 'paid') {
      console.log('Payment received! Preimage:', status.preimage);
      // Grant access to premium features
    } else if (status.status === 'user_rejected') {
      console.log('User rejected payment');
    } else if (status.status === 'timeout') {
      console.log('Payment timed out');
    }
  }
);
```

### Fetch User Profile

```typescript
const userPubkey = 'user-public-key-hex';

const profile = await client.fetchProfile(userPubkey);

if (profile) {
  console.log('Name:', profile.name);
  console.log('Display name:', profile.display_name);
  console.log('Picture:', profile.picture);
  console.log('About:', profile.about);
  console.log('NIP-05:', profile.nip05);
} else {
  console.log('No profile found');
}
```

</section>

<div slot="title">Java</div>
<section>

Use **KeyHandshakeUrlRequest** for auth URLs, **RequestSinglePaymentRequest** for payments, and the appropriate request classes for profiles. Instantiate a request and pass it to `portalSDK.sendCommand(request, (response, err) -> { ... })`.

</section>

</custom-tabs>

## Working with Types

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### Timestamps

```typescript
import { Timestamp } from 'portal-sdk';

// Create from specific date
const specificTime = Timestamp.fromDate(new Date('2024-12-31'));

// Create from now + seconds
const oneHourFromNow = Timestamp.fromNow(3600); // 1 hour
const oneDayFromNow = Timestamp.fromNow(86400); // 24 hours

// Use in payment requests
const paymentRequest = {
  amount: 1000,
  currency: Currency.Millisats,
  expires_at: Timestamp.fromNow(3600)
};
```

### Currency

```typescript
import { Currency } from 'portal-sdk';

// Currently only Millisats is supported
const amount = 1000; // 1 sat = 1000 millisats
const currency = Currency.Millisats;
```

### Profiles

```typescript
import { Profile } from 'portal-sdk';

const profile: Profile = {
  id: 'unique-id',
  pubkey: 'user-public-key',
  name: 'johndoe',
  display_name: 'John Doe',
  picture: 'https://example.com/avatar.jpg',
  about: 'Software developer',
  nip05: 'john@example.com'
};

// Update your service profile
await client.setProfile(profile);
```

</section>

<div slot="title">Java</div>
<section>

Request/response and notification types are in the SDK. Use the request classes (e.g. **CalculateNextOccurrenceRequest**) and handle responses in the `sendCommand` callback. Main types: **PortalRequest**, **PortalResponse**, **PortalNotification**.

</section>

</custom-tabs>

## Error Handling

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### Try-Catch Pattern

```typescript
try {
  await client.connect();
  await client.authenticate('token');
  
  const url = await client.newKeyHandshakeUrl((key) => {
    console.log('User key:', key);
  });
  
} catch (error) {
  if (error.message.includes('timeout')) {
    console.error('Connection timed out');
  } else if (error.message.includes('Authentication failed')) {
    console.error('Invalid auth token');
  } else {
    console.error('Unknown error:', error);
  }
}
```

### Graceful Degradation

```typescript
async function connectWithRetry(maxAttempts = 3) {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      await client.connect();
      return true;
    } catch (error) {
      console.log(`Attempt ${i + 1} failed, retrying...`);
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  return false;
}

const connected = await connectWithRetry();
if (!connected) {
  console.error('Failed to connect after retries');
}
```

</section>

<div slot="title">Java</div>
<section>

Check the `err` parameter in each `sendCommand` callback; handle connection and auth failures before sending commands. See [Error Handling](error-handling.md).

</section>

</custom-tabs>

## Best Practices

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### 1. Reuse Client Instance

```typescript
// ✅ Good - Single instance
const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client.connect();
await client.authenticate('token');

// Use client for multiple operations
const url1 = await client.newKeyHandshakeUrl(handler1);
const url2 = await client.newKeyHandshakeUrl(handler2);
```

```typescript
// ❌ Bad - Multiple instances
const client1 = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client1.connect();

const client2 = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client2.connect();
```

### 2. Handle Cleanup

```typescript
// Server shutdown
process.on('SIGTERM', () => {
  client.disconnect();
  process.exit(0);
});

// Unhandled errors
process.on('unhandledRejection', (error) => {
  console.error('Unhandled rejection:', error);
  client.disconnect();
  process.exit(1);
});
```

### 3. Use Environment Variables

```typescript
// ✅ Good
const client = new PortalSDK({
  serverUrl: process.env.PORTAL_WS_URL || 'ws://localhost:3000/ws'
});
await client.authenticate(process.env.PORTAL_AUTH_TOKEN);
```

```typescript
// ❌ Bad - Hardcoded secrets
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});
await client.authenticate('my-secret-token');
```

### 4. Validate User Input

```typescript
async function authenticateUser(pubkey: string) {
  // Validate pubkey format
  if (!/^[0-9a-f]{64}$/i.test(pubkey)) {
    throw new Error('Invalid pubkey format');
  }
  
  return await client.authenticateKey(pubkey);
}
```

### 5. Log Important Events

```typescript
const client = new PortalSDK({
  serverUrl: process.env.PORTAL_WS_URL
});

client.on({
  onConnected: () => {
    console.log('[Portal] Connected');
  },
  onDisconnected: () => {
    console.log('[Portal] Disconnected');
  },
  onError: (error) => {
    console.error('[Portal] Error:', error);
  }
});
```

</section>

<div slot="title">Java</div>
<section>

Reuse one `PortalSDK` instance, call `connect(authToken)` once, and use `sendCommand` for all operations. See [portal-demo](https://github.com/PortalTechnologiesInc/portal-demo) for a full Kotlin example.

</section>

</custom-tabs>

## Complete Example

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

Here's a complete example integrating authentication and payments:

```typescript
import { PortalSDK, Currency, Timestamp } from 'portal-sdk';

class PortalService {
  private client: PortalSDK;

  constructor(wsUrl: string, authToken: string) {
    this.client = new PortalSDK({ serverUrl: wsUrl });
    this.init(authToken);
  }

  private async init(authToken: string) {
    await this.client.connect();
    await this.client.authenticate(authToken);
    
    this.client.on({
      onDisconnected: () => {
        console.log('Portal disconnected, attempting reconnect...');
        this.init(authToken);
      },
      onError: (error) => {
        console.error('Portal error:', error);
      }
    });
  }

  async createAuthUrl(onAuth: (pubkey: string) => void): Promise<string> {
    return await this.client.newKeyHandshakeUrl(async (pubkey) => {
      const authResult = await this.client.authenticateKey(pubkey);
      if (authResult.status.status === 'approved') {
        onAuth(pubkey);
      }
    });
  }

  async requestPayment(userPubkey: string, amount: number, description: string): Promise<boolean> {
    return new Promise((resolve) => {
      this.client.requestSinglePayment(
        userPubkey,
        [],
        {
          amount,
          currency: Currency.Millisats,
          description
        },
        (status) => {
          if (status.status === 'paid') {
            resolve(true);
          } else if (status.status === 'user_rejected' || status.status === 'timeout') {
            resolve(false);
          }
        }
      );
    });
  }

  async getUserProfile(pubkey: string) {
    return await this.client.fetchProfile(pubkey);
  }

  disconnect() {
    this.client.disconnect();
  }
}

// Usage
const portal = new PortalService(
  process.env.PORTAL_WS_URL!,
  process.env.PORTAL_AUTH_TOKEN!
);

const authUrl = await portal.createAuthUrl((pubkey) => {
  console.log('User authenticated:', pubkey);
});

console.log('Auth URL:', authUrl);
```

</section>

<div slot="title">Java</div>
<section>

```java
var portalSDK = new PortalSDK(healthEndpoint, wsEndpoint);
portalSDK.connect(authToken);

portalSDK.sendCommand(new CalculateNextOccurrenceRequest("weekly", System.currentTimeMillis() / 1000), (res, err) -> {
    if (err != null) {
        logger.error("error: {}", err);
        return;
    }
    logger.info("next occurrence: {}", res.next_occurrence());
});
```

See [portal-demo](https://github.com/PortalTechnologiesInc/portal-demo) for a complete Kotlin example.

</section>

</custom-tabs>

---

**Next Steps**:
- [Configuration](configuration.md) - Advanced configuration options
- [Authentication Guide](../guides/authentication.md) - Deep dive into authentication
- [Payment Processing](../guides/single-payments.md) - Learn about payments

