# SDK Configuration

Configure the Portal TypeScript SDK for your specific needs.

## Basic Configuration

```typescript
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  connectTimeout: 10000
});
```

## Configuration Options

### serverUrl (required)

The WebSocket URL of your Portal daemon.

```typescript
// Local development
serverUrl: 'ws://localhost:3000/ws'

// Production
serverUrl: 'wss://portal.yourdomain.com/ws'
```

### connectTimeout (optional)

Connection timeout in milliseconds. Default: 10000 (10 seconds)

```typescript
connectTimeout: 5000  // 5 seconds
```

## Environment-Based Configuration

```typescript
const config = {
  serverUrl: process.env.PORTAL_WS_URL || 'ws://localhost:3000/ws',
  connectTimeout: parseInt(process.env.PORTAL_TIMEOUT || '10000')
};

const client = new PortalSDK(config);
```

## Event Configuration

Set up event listeners during initialization:

```typescript
const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });

client.on({
  onConnected: () => console.log('Connected'),
  onDisconnected: () => console.log('Disconnected'),
  onError: (error) => console.error('Error:', error)
});

await client.connect();
```

---

**Next**: [Error Handling](error-handling.md)

