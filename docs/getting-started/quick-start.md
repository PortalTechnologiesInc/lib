# Quick Start

Use the SDK with an existing Portal endpoint, or run Portal locally with Docker.

## 0. Requirements

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

- Node.js 18+
- A Portal endpoint (URL) and auth token (from a hosted Portal or [run locally](#2-endpoint-and-token))

</section>

<div slot="title">Java</div>
<section>

- Java 17+
- A Portal endpoint (URL) and auth token (from a hosted Portal or [run locally](#2-endpoint-and-token))

</section>

</custom-tabs>

## 1. Install SDK

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

Or yarn/pnpm. See [SDK Installation](../sdk/installation.md).

</section>

<div slot="title">Java</div>
<section>

**Gradle** (build.gradle or build.gradle.kts):

```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.2.1'
}
```

**Maven** (pom.xml):

```xml
<repository>
    <id>jitpack.io</id>
    <url>https://jitpack.io</url>
</repository>
<dependency>
    <groupId>com.github.PortalTechnologiesInc</groupId>
    <artifactId>java-sdk</artifactId>
    <version>0.2.1</version>
</dependency>
```

See [SDK Installation](../sdk/installation.md).

</section>

</custom-tabs>

## 2. Endpoint and token

**You have them:** Use as serverUrl and in authenticate(token).

**Run locally (Docker):** You need a Nostr private key (hex). Then:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=my-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Check: curl http://localhost:3000/health → OK. Use ws://localhost:3000/ws and token my-secret-token.

## 3. Connect and first flow

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```javascript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client.connect();
await client.authenticate('my-secret-token');

const url = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('User key:', mainKey);
});
console.log('Share with user:', url);
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.PortalSDK;
import cc.getportal.command.request.KeyHandshakeUrlRequest;
import cc.getportal.command.response.KeyHandshakeUrlResponse;
import cc.getportal.command.notification.KeyHandshakeUrlNotification;

PortalSDK sdk = new PortalSDK("ws://localhost:3000/ws");
sdk.connect();
sdk.authenticate("my-secret-token");

sdk.sendCommand(
    new KeyHandshakeUrlRequest((n) -> {
        System.out.println("mainKey: " + n.main_key());
    }),
    (res, err) -> {
        if (err != null) {
            System.err.println("error: " + err);
            return;
        }
        System.out.println("handshake URL: " + res.url());
    }
);
```

See [API Reference](../sdk/api-reference.md) and [Authentication guide](../guides/authentication.md).

</section>

</custom-tabs>

## Common issues

| Issue | Fix |
|-------|-----|
| Connection refused | Portal not running or wrong URL. For Docker: `docker ps`, use `ws://localhost:3000/ws`. |
| Auth failed | Token must match `PORTAL__AUTH__AUTH_TOKEN` used when starting Portal. |
| Invalid Nostr key | Use hex; convert nsec with e.g. `nak decode nsec ...`. |

---

- [Basic usage](../sdk/basic-usage.md) · [Authentication](../guides/authentication.md) · [Single payments](../guides/single-payments.md) · [Troubleshooting](../advanced/troubleshooting.md)
