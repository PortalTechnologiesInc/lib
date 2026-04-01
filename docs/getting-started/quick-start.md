# Quick Start

Use Portal from any language over HTTP, or install an SDK for JavaScript or Java.

## 1. Run the Portal daemon

You need a Nostr private key (hex) and a secret auth token. Then:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=my-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:0.4.1
```

Check it's running:
```bash
curl http://localhost:3000/health
# → OK
```

## 2. Install (optional — only if using an SDK)

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

Nothing to install. Set your base URL and token:

```bash
export BASE_URL=http://localhost:3000
export AUTH_TOKEN=my-secret-token
```

</section>

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

- Node.js 18+
- See [SDK Installation](../sdk/installation.md).

</section>

<div slot="title">Java</div>
<section>

**Gradle:**

```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.4.1'
}
```

**Maven:**

```xml
<repository>
    <id>jitpack.io</id>
    <url>https://jitpack.io</url>
</repository>
<dependency>
    <groupId>com.github.PortalTechnologiesInc</groupId>
    <artifactId>java-sdk</artifactId>
    <version>0.4.1</version>
</dependency>
```

See [SDK Installation](../sdk/installation.md).

</section>

</custom-tabs>

## 3. First request — key handshake URL

Generate a URL for a user to authenticate with their Nostr wallet:

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
# Start the handshake — get a URL to show the user
curl -s -X POST $BASE_URL/key-handshake \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# → { "stream_id": "abc123", "url": "nostr+walletconnect://..." }

# Show the URL to the user (QR code, link, etc.)
# Then poll for the user's public key:
curl -s "$BASE_URL/events/abc123?after=0" \
  -H "Authorization: Bearer $AUTH_TOKEN"
# → { "events": [{ "index": 0, "data": { "main_key": "USER_PUBKEY_HEX" } }] }
```

See [REST API](../sdk/rest-api.md) for the full async polling pattern.

</section>

<div slot="title">JavaScript</div>
<section>

```javascript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: 'my-secret-token'
});

const { url, stream } = await client.newKeyHandshakeUrl();
console.log('Share with user:', url);

// Wait for the user to scan/click the URL
const result = await client.poll(stream);
console.log('User key:', result.main_key);
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.PortalClient;
import cc.getportal.PortalClientConfig;

PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "my-secret-token")
);

var operation = client.newKeyHandshakeUrl();
System.out.println("Share with user: " + operation.url());

// Poll until the user completes the handshake
var result = client.pollUntilComplete(operation);
System.out.println("User key: " + result.main_key());
```

See [API Reference](../sdk/api-reference.md) and [Authentication guide](../guides/authentication.md).

</section>

</custom-tabs>

## Common issues

| Issue | Fix |
|-------|-----|
| Connection refused | Portal not running or wrong port. Check `docker ps`. |
| 401 Unauthorized | Token must match `PORTAL__AUTH__AUTH_TOKEN`. |
| Invalid Nostr key | Use hex; convert nsec with e.g. `nak decode nsec ...`. |

---

- [REST API](../sdk/rest-api.md) · [Authentication](../guides/authentication.md) · [Single payments](../guides/single-payments.md) · [Troubleshooting](../advanced/troubleshooting.md)
