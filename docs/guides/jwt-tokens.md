# JWT Tokens (Session Management)

Verify JWTs issued by user wallets for API auth. Typically the wallet issues the token after authentication; you verify it.

## API

- **verifyJwt(publicKey, token):** Returns target_key; throws if invalid or expired.
- **issueJwt(targetKey, durationHours):** Issue a JWT (e.g. for service-to-service); less common than verification.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
const claims = await client.verifyJwt(servicePublicKey, tokenFromUser);
// claims.target_key — user identity
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.IssueJwtRequest;
import cc.getportal.command.request.VerifyJwtRequest;
import cc.getportal.command.response.IssueJwtResponse;
import cc.getportal.command.response.VerifyJwtResponse;

// issueJwt
sdk.sendCommand(
    new IssueJwtRequest("target-pubkey-hex", 24L),
    (res, err) -> {
        if (err != null) return;
        System.out.println("jwt: " + res.token());
    }
);

// verifyJwt
sdk.sendCommand(
    new VerifyJwtRequest("pubkey-hex", "jwt-token-string"),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("valid: " + res);
    }
);
```

</section>

</custom-tabs>

---

**Next:** [Relay Management](relays.md)
