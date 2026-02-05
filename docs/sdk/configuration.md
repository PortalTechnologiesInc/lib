# SDK Configuration

Configure the Portal SDK for your specific needs.

## Basic Configuration

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  connectTimeout: 10000
});
```

</section>

<div slot="title">Java</div>
<section>

Create `PortalSDK` with the WebSocket endpoint: `new PortalSDK(wsEndpoint)`. Call `connect()` then `authenticate(authToken)`.

</section>

</custom-tabs>

## Configuration Options

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### serverUrl (required)

The Portal endpoint URL (where Portal is running—e.g. your hosted URL or local Docker).

```typescript
// Local development (e.g. Docker)
serverUrl: 'ws://localhost:3000/ws'

// Production (hosted Portal)
serverUrl: 'wss://portal.yourdomain.com/ws'
```

### connectTimeout (optional)

Connection timeout in milliseconds. Default: 10000 (10 seconds)

```typescript
connectTimeout: 5000  // 5 seconds
```

### debug (optional)

When `true`, the SDK logs requests and responses to the console (via `console.debug`). Useful for development. Default: `false`.

```typescript
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  debug: true
});
```

</section>

<div slot="title">Java</div>
<section>

Pass WebSocket URL from environment or config to `new PortalSDK(wsUrl)`; pass auth token to `authenticate(authToken)` after `connect()`.

</section>

</custom-tabs>

## Environment-Based Configuration

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
const config = {
  serverUrl: process.env.PORTAL_WS_URL || 'ws://localhost:3000/ws',
  connectTimeout: parseInt(process.env.PORTAL_TIMEOUT || '10000')
};

const client = new PortalSDK(config);
```

</section>

</custom-tabs>

## Event Configuration

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

</custom-tabs>

---

**Next**: [Error Handling](error-handling.md)

