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

```java
import cc.getportal.PortalSDK;
import cc.getportal.command.request.KeyHandshakeUrlRequest;
import cc.getportal.command.response.KeyHandshakeUrlResponse;
import cc.getportal.command.notification.KeyHandshakeUrlNotification;

PortalSDK sdk = new PortalSDK("ws://localhost:3000/ws");
sdk.connect();
sdk.authenticate("my-secret-token");

sdk.sendCommand(
    new KeyHandshakeUrlRequest((n) ->
        System.out.println("mainKey: " + n.main_key())),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("URL: " + res.url());
    }
);
```

Other requests: RequestSinglePaymentRequest, MintCashuRequest, CalculateNextOccurrenceRequest, etc. See [API Reference](api-reference.md).

</section>

</custom-tabs>

## What to call

- **Auth:** `newKeyHandshakeUrl`, `authenticateKey`
- **Payments:** `requestSinglePayment`, `requestRecurringPayment`, `requestInvoicePayment` — see [Guides](../guides/authentication.md)
- **Profiles:** `fetchProfile`, `setProfile`

Types (Currency, Timestamp, Profile, request/response types) are in the package; use the [API Reference](api-reference.md) and your editor’s types.

---

- [Configuration](configuration.md) · [Error Handling](error-handling.md) · [Guides](../guides/authentication.md)
