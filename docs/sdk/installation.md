# TypeScript SDK Installation

Install and set up the Portal TypeScript SDK in your project.

## Installation

### Using npm

```bash
npm install portal-sdk
```

### Using yarn

```bash
yarn add portal-sdk
```

### Using pnpm

```bash
pnpm add portal-sdk
```

## Requirements

- **Node.js**: 18.x or higher
- **TypeScript** (optional): 4.5 or higher
- **Portal endpoint and auth token**: From a hosted Portal or from [running Portal yourself](../getting-started/docker-deployment.md) (e.g. Docker)

## Verify Installation

Create a test file to verify the installation:

```typescript
import { PortalSDK } from 'portal-sdk';

console.log('Portal SDK imported successfully!');
```

Run it:
```bash
node test.js
```

## TypeScript Support

The SDK includes full TypeScript definitions. No additional `@types` packages are needed.

### tsconfig.json Setup

Recommended TypeScript configuration:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "moduleResolution": "node",
    "resolveJsonModule": true
  }
}
```

## Import Options

### ES Modules

```typescript
import { PortalSDK, Currency, Timestamp } from 'portal-sdk';
```

### CommonJS

```javascript
const { PortalSDK, Currency, Timestamp } = require('portal-sdk');
```

### Import Individual Types

```typescript
import { 
  PortalSDK, 
  Currency, 
  Timestamp,
  Profile,
  AuthResponseData,
  InvoiceStatus,
  RecurringPaymentRequestContent,
  SinglePaymentRequestContent
} from 'portal-sdk';
```

## Browser Support

The SDK works in both Node.js and browser environments.

### Browser Setup

```html
<!DOCTYPE html>
<html>
<head>
  <title>Portal SDK Example</title>
</head>
<body>
  <script type="module">
    import { PortalSDK } from './node_modules/portal-sdk/dist/index.js';
    
    const client = new PortalSDK({
      serverUrl: 'ws://localhost:3000/ws'
    });
    
    // Your code here
  </script>
</body>
</html>
```

### Webpack Configuration

If using Webpack, you may need to configure WebSocket:

```javascript
// webpack.config.js
module.exports = {
  resolve: {
    fallback: {
      "ws": false
    }
  }
};
```

### Browser Bundlers

The SDK uses `isomorphic-ws` which automatically handles WebSocket in both Node.js and browser environments. Most modern bundlers (Vite, Rollup, esbuild) will handle this automatically.

## Framework Integration

### React

```tsx
import React, { useEffect, useState } from 'react';
import { PortalSDK } from 'portal-sdk';

function App() {
  const [client, setClient] = useState<PortalSDK | null>(null);

  useEffect(() => {
    const portalClient = new PortalSDK({
      serverUrl: 'ws://localhost:3000/ws'
    });

    portalClient.connect().then(() => {
      portalClient.authenticate('your-auth-token').then(() => {
        setClient(portalClient);
      });
    });

    return () => {
      portalClient.disconnect();
    };
  }, []);

  return (
    <div>
      {client ? 'Connected to Portal' : 'Connecting...'}
    </div>
  );
}
```

### Next.js

```typescript
// lib/portal.ts
import { PortalSDK } from 'portal-sdk';

let client: PortalSDK | null = null;

export function getPortalClient() {
  if (!client) {
    client = new PortalSDK({
      serverUrl: process.env.NEXT_PUBLIC_PORTAL_WS_URL || 'ws://localhost:3000/ws'
    });
  }
  return client;
}
```

Use in API route:
```typescript
// pages/api/auth.ts
import { getPortalClient } from '@/lib/portal';

export default async function handler(req, res) {
  const client = getPortalClient();
  await client.connect();
  await client.authenticate(process.env.PORTAL_AUTH_TOKEN);
  
  // Use client...
  
  res.status(200).json({ success: true });
}
```

### Vue.js

```typescript
// plugins/portal.ts
import { PortalSDK } from 'portal-sdk';

export default defineNuxtPlugin(() => {
  const client = new PortalSDK({
    serverUrl: 'ws://localhost:3000/ws'
  });

  return {
    provide: {
      portal: client
    }
  };
});
```

### Express.js

```typescript
import express from 'express';
import { PortalSDK } from 'portal-sdk';

const app = express();

// Initialize Portal client
const portalClient = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

// Connect on server start
portalClient.connect().then(() => {
  return portalClient.authenticate(process.env.PORTAL_AUTH_TOKEN);
}).then(() => {
  console.log('Portal SDK connected');
});

// Use in routes
app.post('/api/authenticate', async (req, res) => {
  const url = await portalClient.newKeyHandshakeUrl((mainKey) => {
    console.log('User authenticated:', mainKey);
    // Create user session...
  });
  
  res.json({ authUrl: url });
});

app.listen(3001, () => {
  console.log('Server running on port 3001');
});
```

## Environment Variables

Store your Portal configuration in environment variables:

```bash
# .env
PORTAL_WS_URL=ws://localhost:3000/ws
PORTAL_AUTH_TOKEN=your-secret-auth-token
```

Access in your code:

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: process.env.PORTAL_WS_URL || 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.PORTAL_AUTH_TOKEN || '');
```

## Package Information

### Exports

The package exports the following:

- `PortalSDK` - Main client class
- `Currency` - Currency enum
- `Timestamp` - Timestamp utility class
- All TypeScript types and interfaces

### Bundle Size

- **Minified**: ~50KB
- **Minified + Gzipped**: ~15KB

### Dependencies

The SDK has minimal dependencies:
- `ws` - WebSocket client for Node.js
- `isomorphic-ws` - Universal WebSocket wrapper

## Troubleshooting

### "Cannot find module 'portal-sdk'"

```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
```

### TypeScript Errors

```bash
# Ensure TypeScript is installed
npm install --save-dev typescript

# Check your tsconfig.json includes the right settings
```

### Connection Issues

```bash
# Verify Portal is running (if self-hosting)
curl http://localhost:3000/health

# Check endpoint URL (ws:// for local, wss:// for hosted)
```

### Module Resolution Errors

If using ES modules, ensure your `package.json` has:

```json
{
  "type": "module"
}
```

Or use `.mjs` file extension:
```bash
mv app.js app.mjs
```

## Next Steps

- [Basic Usage](basic-usage.md) - Learn how to use the SDK
- [Configuration](configuration.md) - Configure the SDK
- [Authentication Guide](../guides/authentication.md) - Implement authentication

---

**Ready to start coding?** Head to [Basic Usage](basic-usage.md)

