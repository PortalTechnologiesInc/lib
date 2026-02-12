# Cashu Tokens (Tickets)

Cashu ecash tokens: mint, send, request, and burn. Use for tickets, vouchers, or transferable access. Tokens are backed by sats at a mint; you request from users or mint and send.

## API

- **requestCashu(mainKey, subkeys, mintUrl, unit, amount):** Request tokens from a user. Returns status (success / insufficient_funds / rejected), token, reason. On success, burn the token to claim.
- **mintCashu(mintUrl, staticAuthToken?, unit, amount, description?):** Mint new tokens. Returns token string.
- **burnCashu(mintUrl, unit, token, staticAuthToken?):** Burn (redeem) a token. Returns amount in millisats.
- **sendCashuDirect(mainKey, subkeys, token):** Send a token to a user.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
// Request from user
const result = await client.requestCashu(userPubkey, [], mintUrl, 'sat', 10000);
if (result.status === 'success') {
  const amount = await client.burnCashu(mintUrl, 'sat', result.token);
}

// Mint and send to user
const token = await client.mintCashu(mintUrl, undefined, 'sat', 10000, 'Ticket');
await client.sendCashuDirect(userPubkey, [], token);
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.RequestCashuRequest;
import cc.getportal.command.request.MintCashuRequest;
import cc.getportal.command.request.BurnCashuRequest;
import cc.getportal.command.request.SendCashuDirectRequest;
import java.util.List;

// requestCashu
sdk.sendCommand(
    new RequestCashuRequest("https://mint.example.com", "sat", 1000L, "recipient-pubkey", List.of()),
    (res, err) -> { if (err == null) System.out.println(res); }
);

// mintCashu
sdk.sendCommand(
    new MintCashuRequest("https://mint.example.com", null, "sat", 500L, "tip"),
    (res, err) -> { if (err == null) System.out.println(res); }
);

// burnCashu
sdk.sendCommand(
    new BurnCashuRequest("https://mint.example.com", null, "sat", "cashu-token-string"),
    (res, err) -> { if (err == null) System.out.println(res); }
);

// sendCashuDirect
sdk.sendCommand(
    new SendCashuDirectRequest("user-pubkey-hex", List.of(), "cashu-token-string"),
    (res, err) -> { if (err == null) System.out.println(res); }
);
```

</section>

</custom-tabs>

Burn tokens immediately after receiving to prevent reuse. For your own mint and custom units, see [Running a Mint](running-a-mint.md). Public mints: e.g. minibits.cash, bitcoinmints.com — see [bitcoinmints.com](https://bitcoinmints.com/).

---

**Next:** [JWT Tokens](jwt-tokens.md) · [Single Payments](single-payments.md)
