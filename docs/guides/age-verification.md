# Age Verification

Verify a user's age through Portal's browser-based verification service. The flow uses Cashu tokens as cryptographic proof — tokens have no monetary value, they serve as tamper-proof verification tickets.

## How it works

1. Your backend creates a verification session → gets a `session_url`
2. Redirect the user to the `session_url` in their browser
3. The user completes identity verification
4. Portal mints a Cashu verification token and returns it via the event stream
5. Your backend receives the token — verification complete ✅

The entire flow is handled by a single SDK call (`createVerificationSession`), which creates the session and automatically starts listening for the token.

## Prerequisites

- A **PortalHub** account at [hub.getportal.cc](https://hub.getportal.cc) — create your verification API key and manage your dashboard from there
- `[verification] api_key` configured in portal-rest
- A wallet configured (NWC or Breez) — needed for relay connectivity

## Configuration

1. Sign up / log in at [hub.getportal.cc](https://hub.getportal.cc)
2. Create a verification API key from the dashboard
3. Add it to your `config.toml`:

```toml
[verification]
api_key = "your-api-key"
```

Or via environment variable:

```bash
PORTAL__VERIFICATION__API_KEY=your-api-key
```

## Creating a verification session

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
# Create session (relays are optional — defaults to [nostr] config)
curl -s -X POST $BASE_URL/verification/sessions \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# → {
#   "session_id": "abc-123",
#   "session_url": "https://verify.getportal.cc/?id=abc-123",
#   "ephemeral_npub": "npub1...",
#   "expires_at": 1234567890,
#   "stream_id": "def-456"
# }

# Poll for the verification token
curl -s "$BASE_URL/events/def-456" \
  -H "Authorization: Bearer $AUTH_TOKEN"
# → cashu_response event with the token when verification completes
```

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
import { PortalClient } from 'portal-sdk';

const client = new PortalClient({
  baseUrl: 'http://localhost:3000',
  authToken: 'your-token',
});

// Single call — creates session + listens for token
const session = await client.createVerificationSession();

console.log(`Redirect user to: ${session.session_url}`);

// Wait for the user to complete verification
const result = await client.poll(session, {
  intervalMs: 1000,
  timeoutMs: 5 * 60 * 1000,
});

if (result.status === 'success') {
  console.log('Verified!', result.token);
} else {
  console.log('Failed:', result);
}
```

</section>

</custom-tabs>

## Custom relays

By default, the session uses the relays from your `[nostr]` config. Override per-request:

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
curl -s -X POST $BASE_URL/verification/sessions \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "relays": ["wss://relay.damus.io"] }'
```

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
const session = await client.createVerificationSession([
  'wss://relay.damus.io',
]);
```

</section>

</custom-tabs>

## Requesting a token from a verified user

If a user already holds a verification token (e.g. verified through the mobile app), you can request it directly:

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
# Poll events for cashu_response
```

</section>

<div slot="title">JavaScript</div>
<section>

```typescript
const op = await client.requestVerificationToken(userPubkeyHex, []);
const result = await client.poll(op, { intervalMs: 1000, timeoutMs: 60_000 });
```

</section>

</custom-tabs>

## Token lifecycle

- **Web verification** tokens have an amount of 1 (single-use ticket)
- **Mobile app** tokens have an amount of 500 (reusable across services)
- Tokens use Portal's mint (`https://mint.getportal.cc`) with unit `multi`
- Cashu is used purely as a protocol — tokens carry no monetary value
- To prevent replay attacks, **burn the token** after receiving it (see [Cashu Tokens guide](cashu-tokens.md))

## Verification statuses

| Status | Description |
|--------|-------------|
| `success` | Verification passed. `token` field contains the Cashu token. |
| `rejected` | Verification failed. `reason` may contain details. |
| `insufficient_funds` | Mint could not issue the token. |

---

- [Cashu Tokens](cashu-tokens.md) · [Environment Variables](../getting-started/environment-variables.md) · [API Reference](../sdk/api-reference.md)
