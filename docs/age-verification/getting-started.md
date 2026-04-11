# Age Verification — Getting Started

Add age verification to your app in 5 minutes. Users verify once, you get a cryptographic proof — no personal data stored.

## How it works

1. Your backend creates a **verification session** → gets a URL
2. You redirect the user to that URL
3. The user completes identity verification in their browser
4. Your backend receives a **verification proof** — done ✅

## 1. Create a PortalHub account

Sign up at [hub.getportal.cc](https://hub.getportal.cc).

## 2. Create a Portal instance

From the PortalHub dashboard, create a new **Portal instance**. PortalHub hosts and runs it for you — no Docker, no servers, no setup.

You'll get:
- An **instance URL** (e.g. `https://your-instance.hub.getportal.cc`)
- An **API auth token**

## 3. Create a verification API key

In the same dashboard, create a **verification API key** for your instance. This is what authorizes verification sessions.

## 4. Your first verification

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
export BASE_URL=https://your-instance.hub.getportal.cc
export AUTH_TOKEN=your-api-auth-token

# Step 1: Create a verification session
curl -s -X POST $BASE_URL/verification/sessions \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# → {
#   "session_url": "https://verify.getportal.cc/?id=abc-123",
#   "stream_id": "def-456",
#   ...
# }

# Step 2: Redirect the user to session_url in their browser

# Step 3: Poll for the verification result
curl -s "$BASE_URL/events/def-456" \
  -H "Authorization: Bearer $AUTH_TOKEN"
# → When the user completes verification, you'll receive the proof
```

</section>

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'https://your-instance.hub.getportal.cc',
  authToken: 'your-api-auth-token',
});

// Create session and wait for verification
const session = await client.createVerificationSession();
console.log(`Redirect user to: ${session.session_url}`);

// Poll until the user completes verification
const result = await client.poll(session, {
  intervalMs: 1000,
  timeoutMs: 5 * 60 * 1000, // 5 minute timeout
});

if (result.status === 'success') {
  console.log('User is verified!', result.token);
} else {
  console.log('Verification failed:', result);
}
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.PortalClient;
import cc.getportal.PortalClientConfig;

PortalClient client = new PortalClient(
    PortalClientConfig.create(
        "https://your-instance.hub.getportal.cc",
        "your-api-auth-token"
    )
);

// Create verification session
var session = client.createVerificationSession();
System.out.println("Redirect user to: " + session.sessionUrl());

// Poll until verification completes
var result = client.pollUntilComplete(session);
System.out.println("Verified: " + result);
```

</section>

</custom-tabs>

## 5. Done!

That's all you need for basic age verification. Your flow is:

1. User clicks "Verify age" on your site
2. Your backend creates a session → gets `session_url`
3. Redirect user to `session_url`
4. Poll for result → receive verification proof
5. Grant access

## Self-hosting

Want to run your own Portal instance instead of using PortalHub? See [Docker Deployment](../advanced/docker-deployment.md) and [Building from Source](../advanced/building-from-source.md).

## What's next?

- **[Integration Guide](integration-guide.md)** — Web and mobile flows, error handling, full Express.js example
- **[API Reference](api-reference.md)** — Detailed endpoint documentation
