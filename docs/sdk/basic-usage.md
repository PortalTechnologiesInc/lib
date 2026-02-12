# Basic Usage

## Quick example

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
await client.connect();
await client.authenticate('your-auth-token');

const authUrl = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('User key:', mainKey);
});
```

</section>

<div slot="title">Java</div>
<section>

Same flow: `connect()`, `authenticate(authToken)`, then `sendCommand(request, callback)`. Request types: **KeyHandshakeUrlRequest**, **RequestSinglePaymentRequest**, **MintCashuRequest**, etc. See [API Reference](api-reference.md).

</section>

</custom-tabs>

## What to call

- **Auth:** `newKeyHandshakeUrl(onKeyHandshake)`, `authenticateKey(mainKey, subkeys?)`
- **Payments:** `requestSinglePayment`, `requestRecurringPayment`, `requestInvoicePayment` — see [Guides](../guides/authentication.md)
- **Profiles:** `fetchProfile(mainKey)`, `setProfile(profile)`

Types (`Currency`, `Timestamp`, `Profile`, request/response types) are in the package; use the [API Reference](api-reference.md) and your editor’s types.

---

- [Configuration](configuration.md) · [Error Handling](error-handling.md) · [Guides](../guides/authentication.md)
