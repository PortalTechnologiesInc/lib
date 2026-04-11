# Platform — Getting Started

Set up Portal for authentication, payments, profiles, and more. This guide gets you running in minutes.

> **Just need age verification?** See the [Age Verification Quick Start](../age-verification/getting-started.md) instead — it's simpler.

## 1. Get a Portal instance

### Option A: PortalHub (recommended)

Sign up at [hub.getportal.cc](https://hub.getportal.cc) and create a Portal instance. PortalHub hosts and runs it for you — no servers needed.

You'll get:
- An **instance URL** (e.g. `https://your-instance.hub.getportal.cc`)
- An **API auth token**

### Option B: Self-host with Docker

If you prefer to run your own instance:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=my-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=$(openssl rand -hex 32) \
  getportal/sdk-daemon:0.4.1
```

Check it's running:
```bash
curl http://localhost:3000/health
# → OK
```

See [Docker Deployment](../advanced/docker-deployment.md) for production setup.

## 3. Install an SDK (optional)

Portal exposes a standard HTTP REST API — you can use any language. SDKs add convenience.

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

Node.js 18+ required.

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

</section>

</custom-tabs>

## 4. First request — authenticate a user

Generate a URL for a user to log in:

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

```typescript
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

var result = client.pollUntilComplete(operation);
System.out.println("User key: " + result.main_key());
```

</section>

</custom-tabs>

## What's next?

- **[Authentication](authentication.md)** — Full auth flow with subkeys and static tokens
- **[Single Payments](single-payments.md)** — Accept one-time payments
- **[Recurring Payments](recurring-payments.md)** — Set up subscriptions
- **[Docker Deployment](../advanced/docker-deployment.md)** — Production deployment
- **[Environment Variables](../advanced/environment-variables.md)** — All configuration options

## Common issues

| Issue | Fix |
|-------|-----|
| Connection refused | Portal not running or wrong port. Check `docker ps`. |
| 401 Unauthorized | Token must match `PORTAL__AUTH__AUTH_TOKEN`. |
| Invalid Nostr key | Use hex (64 chars); convert nsec with `nak decode nsec1...`. |

---

**Troubleshooting:** [Full troubleshooting guide](../resources/troubleshooting.md)
