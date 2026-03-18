# Configuration

## Client options

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

No client to configure — just set your base URL and auth token in each request:

```bash
export BASE_URL=http://localhost:3000
export AUTH_TOKEN=your-secret-token

# All requests:
curl -s $BASE_URL/endpoint \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json"
```

See [Environment Variables](../getting-started/environment-variables.md) for daemon configuration.

</section>

<div slot="title">JavaScript</div>
<section>

Pass options to `PortalClient`:

| Option | Required | Description |
|--------|----------|-------------|
| `baseUrl` | Yes | HTTP base URL (e.g. `http://localhost:3000`) |
| `authToken` | Yes | Bearer token matching `PORTAL__AUTH__AUTH_TOKEN` |
| `autoPollingIntervalMs` | No | Enable auto-polling; interval in ms (e.g. `2000`) |

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: 'your-auth-token',
  autoPollingIntervalMs: 2000  // optional: poll every 2s automatically
});

// Stop auto-polling when done
client.destroy();
```

</section>

<div slot="title">Java</div>
<section>

Use `PortalClientConfig` builder:

```java
import cc.getportal.PortalClient;
import cc.getportal.PortalClientConfig;

// Manual polling
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
);

// Auto-polling every 2 seconds
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
                      .autoPolling(2000)
);

// Webhook mode
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
                      .webhookSecret("your-webhook-secret")
);
```

</section>

</custom-tabs>

---

**Next:** [Error Handling](error-handling.md)
