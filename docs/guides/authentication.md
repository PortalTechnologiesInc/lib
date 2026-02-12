# Authentication Flow

Portal uses Nostr key pairs: users prove identity by signing challenges. Your app gets an auth URL; the user opens it in a Nostr wallet and approves; you receive the key handshake and verify.

## API

1. **Generate auth URL:** `newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?)` — callback runs when the user completes the handshake.
2. **Authenticate the key:** `authenticateKey(mainKey, subkeys?)` — returns AuthResponseData with status (approved / declined), optional session_token, reason.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
const authUrl = await client.newKeyHandshakeUrl(async (mainKey, preferredRelays) => {
  const authResponse = await client.authenticateKey(mainKey);
  if (authResponse.status.status === 'approved') {
    // session_token in authResponse.status.session_token
  }
});
// Share authUrl (QR, link, etc.) with user
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.KeyHandshakeUrlRequest;
import cc.getportal.command.request.AuthenticateKeyRequest;
import cc.getportal.command.response.KeyHandshakeUrlResponse;
import cc.getportal.command.response.AuthenticateKeyResponse;
import java.util.List;

// 1) Get handshake URL (notification gives mainKey when user completes)
sdk.sendCommand(
    new KeyHandshakeUrlRequest((n) ->
        System.out.println("mainKey: " + n.main_key())),
    (res, err) -> {
        if (err != null) return;
        System.out.println("URL: " + res.url());
    }
);

// 2) Authenticate with key (after user completed handshake)
sdk.sendCommand(
    new AuthenticateKeyRequest("user-pubkey-hex", List.of()),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("authenticated");
    }
);
```

</section>

</custom-tabs>

- **Subkeys:** Pass optional subkeys to `authenticateKey` (JS) or `AuthenticateKeyRequest` (Java) for delegated auth.
- **Static token:** Pass a string as second arg to `newKeyHandshakeUrl` (JS) or `KeyHandshakeUrlRequest(staticToken, noRequest, callback)` (Java) for long-lived reusable URLs.
- **No-request mode:** Third arg true (JS) or noRequest = true (Java) — handshake only, no auth challenge.

Check status === 'approved' before granting access. Use session tokens and expiration in your app; the SDK verifies signatures.

---

**Next:** [Single Payments](single-payments.md) · [Profiles](profiles.md) · [JWT Tokens](jwt-tokens.md)
