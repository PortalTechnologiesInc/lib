# Age Verification — Integration Guide

Detailed integration patterns for age verification. Start with [Getting Started](getting-started.md) if you haven't set up yet.

## Web flow (redirect + poll)

The standard integration for websites:

1. User clicks "Verify age" on your site
2. Your backend creates a verification session
3. Redirect the user to the `session_url`
4. Poll the event stream for the result
5. On success, store the verification proof and grant access

### Full Express.js example

```javascript
import express from 'express';
import { PortalClient } from 'portal-sdk';

const app = express();
const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: process.env.PORTAL_AUTH_TOKEN,
});

// Step 1: User clicks "Verify my age"
app.post('/api/verify-age', async (req, res) => {
  const session = await client.createVerificationSession();

  // Store session info (e.g. in your database, keyed by user ID)
  // session.stream_id is what you'll poll later

  // Redirect user to the verification page
  res.json({ redirect_url: session.session_url });
});

// Step 2: Poll for result (call from frontend or background job)
app.get('/api/verify-age/status/:streamId', async (req, res) => {
  try {
    const result = await client.poll(
      { stream_id: req.params.streamId },
      { intervalMs: 1000, timeoutMs: 30_000 }
    );

    if (result.status === 'success') {
      // Store the verification proof in your database
      // Grant access to age-restricted content
      res.json({ verified: true });
    } else {
      res.json({ verified: false, reason: result.reason });
    }
  } catch (err) {
    res.json({ verified: false, reason: 'timeout' });
  }
});

app.listen(8080);
```

## Mobile flow

For mobile apps, you have two options:

### Option A: In-app browser

Open the `session_url` in a WebView or system browser, then poll for the result:

```typescript
const session = await client.createVerificationSession();

// Open in system browser or WebView
Linking.openURL(session.session_url);

// Poll for result in background
const result = await client.poll(session, {
  intervalMs: 2000,
  timeoutMs: 5 * 60 * 1000,
});
```

### Option B: Request token from already-verified user

If the user has already verified through the Portal mobile app, you can request their verification proof directly — no redirect needed:

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
curl -s -X POST $BASE_URL/verification/token \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "recipient_key": "USER_PUBKEY_HEX",
    "subkeys": []
  }'
# → { "stream_id": "..." }
# Poll events for the verification proof
```

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
const op = await client.requestVerificationToken(userPubkeyHex, []);
const result = await client.poll(op, {
  intervalMs: 1000,
  timeoutMs: 60_000,
});
```

</section>

</custom-tabs>

## Handling verification results

Verification results have three possible statuses:

| Status | Description | Action |
|--------|-------------|--------|
| `success` | Verification passed | Store the proof, grant access |
| `rejected` | Verification failed | Show error, offer to retry |
| `insufficient_funds` | Service issue | Retry later or contact support |

On `success`, you receive a verification proof (a cryptographic token). Store it in your database associated with the user.

### Preventing replay attacks

After receiving a verification proof, **redeem it** to prevent reuse:

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
# Redeem the verification proof
curl -s -X POST $BASE_URL/cashu/burn \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "mint_url": "https://mint.getportal.cc",
    "unit": "multi",
    "token": "VERIFICATION_TOKEN_HERE"
  }'
```

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
// Redeem the proof to prevent replay
await client.burnCashu(
  'https://mint.getportal.cc',
  'multi',
  result.token
);
```

</section>

</custom-tabs>

## Error handling

```typescript
try {
  const session = await client.createVerificationSession();
  const result = await client.poll(session, {
    intervalMs: 1000,
    timeoutMs: 5 * 60 * 1000,
  });

  switch (result.status) {
    case 'success':
      // Burn token, grant access
      break;
    case 'rejected':
      // Show user-friendly error
      console.log('Verification failed:', result.reason);
      break;
    default:
      // Unexpected status
      console.log('Unexpected result:', result);
  }
} catch (err) {
  // Network error, timeout, etc.
  console.error('Verification error:', err);
}
```

## Configuration

If you're using **PortalHub** (recommended), all configuration is handled through the dashboard at [hub.getportal.cc](https://hub.getportal.cc). You just need your instance URL and API auth token.

If you're **self-hosting**, see [Environment Variables](../advanced/environment-variables.md) for the full configuration reference. The key settings for age verification:

| Environment variable | Description |
|---------------------|-------------|
| `PORTAL__AUTH__AUTH_TOKEN` | Your API auth token (required) |
| `PORTAL__VERIFICATION__API_KEY` | Your verification API key from [PortalHub](https://hub.getportal.cc) (required) |
| `PORTAL__NOSTR__PRIVATE_KEY` | A 64-char hex key for the daemon (required, generate with `openssl rand -hex 32`) |

---

**Next:** [API Reference](api-reference.md) · [Getting Started](getting-started.md)
