# Static Tokens & Physical Authentication

Static tokens make auth URLs reusable: pass a string as the second argument to `newKeyHandshakeUrl` so the same URL can be used many times (e.g. printed on a table, written to NFC).

## API

`newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?)` — when you pass a staticToken, the returned URL does not expire after one use. Use it for location-specific auth (tables, doors, kiosks). Your callback receives the same token context so you can associate the handshake with a place.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
const staticToken = 'table-14-restaurant-a';
const authUrl = await client.newKeyHandshakeUrl(
  (mainKey, preferredRelays) => {
    // mainKey + staticToken identify who and where
    handleLocationAuth(staticToken, mainKey);
  },
  staticToken
);
// Share authUrl (QR, NFC, link); it can be reused
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.KeyHandshakeUrlRequest;
import cc.getportal.command.response.KeyHandshakeUrlResponse;
import cc.getportal.command.notification.KeyHandshakeUrlNotification;

sdk.sendCommand(
    new KeyHandshakeUrlRequest("my-static-token", null, (n) ->
        System.out.println("mainKey: " + n.main_key())),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("URL: " + res.url());
    }
);
```

</section>

</custom-tabs>

You are responsible for generating QR codes or writing to NFC; the SDK only provides the URL.

---

**Next:** [Single Payments](single-payments.md) · [Authentication](authentication.md)
