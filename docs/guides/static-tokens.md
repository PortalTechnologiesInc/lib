# Static Tokens & Physical Authentication

Static tokens make auth URLs reusable: pass a string as the second argument to `newKeyHandshakeUrl` so the same URL can be used many times (e.g. printed on a table, written to NFC).

## API

`newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?)` — when you pass a **staticToken**, the returned URL does not expire after one use. Use it for location-specific auth (tables, doors, kiosks). Your callback receives the same token context so you can associate the handshake with a place.

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

You are responsible for generating QR codes or writing to NFC; the SDK only provides the URL.

---

**Next:** [Single Payments](single-payments.md) · [Authentication](authentication.md)
