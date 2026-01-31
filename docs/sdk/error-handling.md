# Error Handling

Handle errors gracefully in your Portal integration.

## PortalSDKError and error codes

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

The SDK throws `PortalSDKError` with a `code` property so you can handle cases in code. Always check `err instanceof PortalSDKError` and switch on `err.code`:

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  await client.connect();
  await client.authenticate(token);
} catch (err) {
  if (err instanceof PortalSDKError) {
    switch (err.code) {
      case 'AUTH_FAILED':
        // Invalid or expired token
        break;
      case 'CONNECTION_TIMEOUT':
      case 'CONNECTION_CLOSED':
        // Connection issues — is Portal running? Check serverUrl.
        break;
      case 'NOT_CONNECTED':
        // Call connect() before other methods
        break;
      case 'UNEXPECTED_RESPONSE':
      case 'SERVER_ERROR':
      case 'PARSE_ERROR':
        // Protocol or server error; err.message and optional err.details
        break;
      default:
        break;
    }
  }
  throw err;
}
```

### Error codes

| Code | When |
|------|------|
| `NOT_CONNECTED` | A method was called before `connect()` or after disconnect. |
| `CONNECTION_TIMEOUT` | Connection did not open within `connectTimeout`. |
| `CONNECTION_CLOSED` | Socket closed unexpectedly. |
| `AUTH_FAILED` | Invalid or rejected auth token. |
| `UNEXPECTED_RESPONSE` | Server sent an unexpected response type. |
| `SERVER_ERROR` | Server returned an error (message in `err.message`). |
| `PARSE_ERROR` | Failed to parse a message; optional `err.details`. |

### Background / connection events

Listen for connection and background errors via `on`:

```typescript
client.on({
  onConnected: () => console.log('Connected'),
  onDisconnected: () => console.log('Disconnected'),
  onError: (e) => console.error('Portal error:', e),
});
```

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java PortalSDKError and error codes will be added here.

</section>

</custom-tabs>

## Common error patterns

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### Connection errors

```typescript
try {
  await client.connect();
} catch (error) {
  if (error instanceof PortalSDKError && error.code === 'CONNECTION_TIMEOUT') {
    console.error('Connection timeout - is Portal daemon running?');
  } else if (error.message?.includes('ECONNREFUSED')) {
    console.error('Connection refused - check serverUrl');
  } else {
    console.error('Connection error:', error);
  }
}
```

### Authentication errors

```typescript
try {
  await client.authenticate('token');
} catch (error) {
  if (error instanceof PortalSDKError && error.code === 'AUTH_FAILED') {
    console.error('Invalid auth token');
  } else {
    console.error('Auth error:', error);
  }
}
```

### Payment errors

```typescript
client.requestSinglePayment(userPubkey, [], request, (status) => {
  if (status.status === 'error') {
    console.error('Payment error:', status.reason);
  } else if (status.status === 'user_failed') {
    console.error('Payment failed:', status.reason);
    // Common reasons: insufficient funds, routing failure
  }
});
```

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java common error patterns will be added here.

</section>

</custom-tabs>

## Error Recovery

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### Connection Retry

```typescript
async function connectWithRetry(maxAttempts = 3, delayMs = 1000) {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      await client.connect();
      console.log('Connected successfully');
      return true;
    } catch (error) {
      console.log(`Connection attempt ${attempt} failed`);
      
      if (attempt < maxAttempts) {
        await new Promise(resolve => setTimeout(resolve, delayMs));
      }
    }
  }
  
  console.error('Failed to connect after retries');
  return false;
}
```

### Automatic Reconnection

```typescript
client.on({
  onDisconnected: () => {
    console.log('Disconnected, attempting reconnect...');
    
    setTimeout(async () => {
      try {
        await client.connect();
        await client.authenticate(process.env.AUTH_TOKEN);
        console.log('Reconnected successfully');
      } catch (error) {
        console.error('Reconnection failed:', error);
      }
    }, 5000);
  }
});
```

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java error recovery will be added here.

</section>

</custom-tabs>

## Best Practices

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

1. **Always use try-catch** for async operations
2. **Check status codes** in callbacks
3. **Implement retry logic** for critical operations
4. **Log errors** with context
5. **Show user-friendly messages** to end users

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java best practices will be added here.

</section>

</custom-tabs>

---

**Next**: [Authentication Guide](../guides/authentication.md)

