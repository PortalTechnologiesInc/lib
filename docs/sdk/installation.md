# Installing the SDK

**JavaScript/TypeScript:** [npm](https://www.npmjs.com/package/portal-sdk) · **Java:** [GitHub](https://github.com/PortalTechnologiesInc/java-sdk)

## Requirements

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

- Node.js 18+
- TypeScript 4.5+ (optional)
- Portal endpoint and auth token

</section>

<div slot="title">Java</div>
<section>

- Java 17+
- Portal endpoint and auth token

</section>

</custom-tabs>

## Install

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

Or `yarn add portal-sdk` / `pnpm add portal-sdk`.

</section>

<div slot="title">Java</div>
<section>

**Gradle** (`build.gradle` or `build.gradle.kts`):

```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.2.0'
}
```

**Maven** (`pom.xml`):

```xml
<repository>
    <id>jitpack.io</id>
    <url>https://jitpack.io</url>
</repository>
<dependency>
    <groupId>com.github.PortalTechnologiesInc</groupId>
    <artifactId>java-sdk</artifactId>
    <version>0.2.0</version>
</dependency>
```

</section>

</custom-tabs>

## Use

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

Create a client, connect, then authenticate with your Portal endpoint and token:

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client.connect();
await client.authenticate('your-auth-token');
```

</section>

<div slot="title">Java</div>
<section>

Create a `PortalSDK` with your WebSocket endpoint, then connect and authenticate:

```java
var portalSDK = new PortalSDK(wsEndpoint);
portalSDK.connect();
portalSDK.authenticate(authToken);
```

Use `sendCommand(request, callback)` for commands; request and response types are in the SDK package. See the [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

---

- [Basic Usage](basic-usage.md) · [Configuration](configuration.md) · [Guides](../guides/authentication.md)
