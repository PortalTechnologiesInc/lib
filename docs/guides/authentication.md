# Authentication Flow

Portal uses Nostr key pairs: users prove identity by signing challenges. Your app gets an auth URL; the user opens it in a Nostr wallet and approves; you receive the key handshake and verify.

## API

1. **Generate auth URL:** `newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?)` — callback runs when the user completes the handshake.
2. **Authenticate the key:** `authenticateKey(mainKey, subkeys?)` — returns `AuthResponseData` with `status.status` (`approved` / `declined`), optional `session_token`, `reason`.

```typescript
const authUrl = await client.newKeyHandshakeUrl(async (mainKey, preferredRelays) => {
  const authResponse = await client.authenticateKey(mainKey);
  if (authResponse.status.status === 'approved') {
    // session_token in authResponse.status.session_token
  }
});
// Share authUrl (QR, link, etc.) with user
```

- **Subkeys:** Pass optional `subkeys` to `authenticateKey` for delegated auth.
- **Static token:** Pass a string as second arg to `newKeyHandshakeUrl` for long-lived reusable URLs.
- **No-request mode:** Third arg `true` — handshake only, no auth challenge.

Check `authResponse.status.status === 'approved'` before granting access. Use session tokens and expiration in your app; the SDK verifies signatures.

---

**Next:** [Single Payments](single-payments.md) · [Profiles](profiles.md) · [JWT Tokens](jwt-tokens.md)
