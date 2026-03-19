# Basic Usage

## Quick example

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
export BASE_URL=http://localhost:3000
export AUTH_TOKEN=your-auth-token

# Get a key handshake URL
curl -s -X POST $BASE_URL/key-handshake \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# → { "stream_id": "abc123", "url": "nostr+walletconnect://..." }

# Poll for the user's public key
curl -s "$BASE_URL/events/abc123?after=0" \
  -H "Authorization: Bearer $AUTH_TOKEN"
```

See [REST API](rest-api.md) for the full async flow.

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: 'your-auth-token'
});

const { url, stream } = await client.newKeyHandshakeUrl();
console.log('User key:', (await client.poll(stream)).main_key);
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.PortalClient;
import cc.getportal.PortalClientConfig;

PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
);

var operation = client.newKeyHandshakeUrl();
System.out.println("URL: " + operation.url());
var result = client.pollUntilComplete(operation);
System.out.println("mainKey: " + result.main_key());
```

See [API Reference](api-reference.md) and the [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

## What to call

- **Auth:** `newKeyHandshakeUrl`, `authenticateKey` — see [Authentication](../guides/authentication.md)
- **Payments:** `requestSinglePayment`, `requestRecurringPayment`, `requestInvoicePayment` — see [Guides](../guides/single-payments.md)
- **Profiles:** `fetchProfile`
- **Full reference:** [REST API](rest-api.md) · [OpenAPI](api-reference-rest.md) · [SDK API Reference](api-reference.md)

---

- [Configuration](configuration.md) · [Error Handling](error-handling.md) · [Guides](../guides/authentication.md)
