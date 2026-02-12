# JWT Tokens (Session Management)

Verify JWTs issued by user wallets for API auth. Typically the wallet issues the token after authentication; you verify it.

## API

- **verifyJwt(publicKey, token):** Returns `{ target_key }`; throws if invalid or expired.
- **issueJwt(targetKey, durationHours):** Issue a JWT (e.g. for service-to-service); less common than verification.

```typescript
const claims = await client.verifyJwt(servicePublicKey, tokenFromUser);
// claims.target_key — user identity
```

---

**Next:** [Relay Management](relays.md)
