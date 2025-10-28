# Error Handling

Handle errors gracefully in your Portal integration.

## Common Error Types

### Connection Errors

```typescript
try {
  await client.connect();
} catch (error) {
  if (error.message.includes('timeout')) {
    console.error('Connection timeout - is Portal daemon running?');
  } else if (error.message.includes('ECONNREFUSED')) {
    console.error('Connection refused - check serverUrl');
  } else {
    console.error('Connection error:', error);
  }
}
```

### Authentication Errors

```typescript
try {
  await client.authenticate('token');
} catch (error) {
  if (error.message.includes('Authentication failed')) {
    console.error('Invalid auth token');
  } else {
    console.error('Auth error:', error);
  }
}
```

### Payment Errors

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

## Error Recovery

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

## Best Practices

1. **Always use try-catch** for async operations
2. **Check status codes** in callbacks
3. **Implement retry logic** for critical operations
4. **Log errors** with context
5. **Show user-friendly messages** to end users

---

**Next**: [Authentication Guide](../guides/authentication.md)

