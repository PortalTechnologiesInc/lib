# Installing the SDK

**JavaScript/TypeScript:** [npm](https://www.npmjs.com/package/portal-sdk) · **Java:** [GitHub](https://github.com/PortalTechnologiesInc/java-sdk) · **Any language:** [REST API](rest-api.md)

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

<div slot="title">HTTP</div>
<section>

No SDK needed. Any HTTP client works: curl, Python, Go, Ruby, PHP, etc.

- Portal endpoint and auth token
- That's it.

</section>

</custom-tabs>

## Install

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

Or yarn add portal-sdk / pnpm add portal-sdk.

</section>

<div slot="title">Java</div>
<section>

**Gradle** (build.gradle or build.gradle.kts):

```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.3.0'
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
    <version>0.3.0</version>
</dependency>
```

</section>

<div slot="title">HTTP</div>
<section>

Nothing to install. Set your base URL and token:

```bash
export BASE_URL=http://localhost:3000
export AUTH_TOKEN=your-secret-token
```

</section>

</custom-tabs>

## Use

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

Create a client, connect, then authenticate with your Portal endpoint and token:

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: 'your-auth-token'
});
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
```

See [API Reference](api-reference.md) and the [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

<div slot="title">HTTP</div>
<section>

```bash
# Health check
curl -s $BASE_URL/health

# All requests use Bearer auth
curl -s $BASE_URL/version \
  -H "Authorization: Bearer $AUTH_TOKEN"
```

See [REST API](rest-api.md) for the full reference and async polling pattern.

</section>

</custom-tabs>

---

> **Compatibility:** the SDK `major.minor` version must match the SDK Daemon (`getportal/sdk-daemon`) `major.minor`. Patch versions are independent. See [Versioning & Compatibility](../getting-started/versioning.md).

---

- [Basic Usage](basic-usage.md) · [Configuration](configuration.md) · [Guides](../guides/authentication.md)
