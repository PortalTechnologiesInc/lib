# Installing the SDK

Install and set up the Portal SDK in your project.

## Installation

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

### Requirements

- **Node.js**: 18.x or higher
- **TypeScript** (optional): 4.5 or higher
- **Portal endpoint and auth token**: From a hosted Portal or from [running Portal yourself](../getting-started/docker-deployment.md) (e.g. Docker)

### Verify Installation

Create a test file to verify the installation:

```typescript
import { PortalSDK } from 'portal-sdk';

console.log('Portal SDK imported successfully!');
```

Run it:
```bash
node test.js
```

</section>

<div slot="title">Java</div>
<section>

### Using Gradle

1. Add the Jitpack repository to your `build.gradle`:

```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
```

2. Add the dependency:

```groovy
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.1.0'
}
```

### Using Maven

1. Add the repository to your `pom.xml`:

```xml
<repository>
    <id>jitpack.io</id>
    <url>https://jitpack.io</url>
</repository>
```

2. Add the dependency:

```xml
<dependency>
    <groupId>com.github.PortalTechnologiesInc</groupId>
    <artifactId>java-sdk</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Requirements

- **Java**: 17 or higher
- **Portal endpoint and auth token**: From a hosted Portal or from [running Portal yourself](../getting-started/docker-deployment.md) (e.g. Docker)

</section>

</custom-tabs>

## Import and setup

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### TypeScript Support

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

</section>

<div slot="title">Java</div>
<section>

Create a `PortalSDK` with your health and WebSocket endpoints, then connect with your auth token:

```java
var portalSDK = new PortalSDK(healthEndpoint, wsEndpoint);
portalSDK.connect(authToken);
```

Use `sendCommand(request, callback)` to send commands; request and response types are in the SDK package.

</section>

</custom-tabs>

## Browser Support

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

The Java SDK targets the JVM (Java 17+). For Kotlin examples, see [portal-demo](https://github.com/PortalTechnologiesInc/portal-demo).

</section>

</custom-tabs>

## Framework Integration

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

Wire `PortalSDK` as a bean (e.g. in Spring) and pass health URL, WebSocket URL, and auth token from configuration or environment variables.

</section>

</custom-tabs>

## Environment Variables

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

Read health URL, WebSocket URL, and auth token from environment or config and pass them to `PortalSDK` and `connect()`.

</section>

</custom-tabs>

## Package Information

### Exports

The package exports the following:

- `PortalSDK` - Main client class
- `Currency` - Currency enum
- `Timestamp` - Timestamp utility class
- All type definitions (TypeScript)

### Bundle Size

- **Minified**: ~50KB
- **Minified + Gzipped**: ~15KB

### Dependencies

The SDK has minimal dependencies:
- `ws` - WebSocket client for Node.js
- `isomorphic-ws` - Universal WebSocket wrapper

## Troubleshooting

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

### "Cannot find module 'portal-sdk'"

```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
```

### TypeScript / module errors

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

</section>

<div slot="title">Java</div>
<section>

- Ensure Jitpack is in `repositories` and the dependency version is correct.
- Verify health and WebSocket URLs and auth token; use `curl` on the health endpoint.
- For issues, see the [Java SDK repository](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

## Next Steps

- [Basic Usage](basic-usage.md) - Learn how to use the SDK
- [Configuration](configuration.md) - Configure the SDK
- [Authentication Guide](../guides/authentication.md) - Implement authentication

---

**Ready to start coding?** Head to [Basic Usage](basic-usage.md)

