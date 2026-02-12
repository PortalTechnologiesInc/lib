# Single Payments

One-time Lightning payments from an authenticated user. You request a payment; the user approves or rejects in their wallet; you get status updates and, on success, a preimage.

## API

`requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange)`

- **paymentRequest:** amount (millisats; 1 sat = 1000), currency (`Currency.Millisats`), description.
- **onStatusChange:** callback receives status objects. status values: paid, user_approved, user_rejected, user_failed, timeout, error. On paid use preimage; on failure use reason.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
await client.requestSinglePayment(
  userPubkey,
  [],
  {
    amount: 10000,  // 10 sats (millisats)
    currency: Currency.Millisats,
    description: 'Premium - 1 month'
  },
  (status) => {
    if (status.status === 'paid') { /* preimage in status.preimage */ }
  }
);
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.RequestSinglePaymentRequest;
import cc.getportal.model.SinglePaymentRequestContent;
import cc.getportal.model.Currency;
import java.util.List;

SinglePaymentRequestContent payment = new SinglePaymentRequestContent(
    "Premium - 1 month", 10_000L, Currency.MILLISATS, null, null
);
sdk.sendCommand(
    new RequestSinglePaymentRequest(
        "user-pubkey-hex",
        List.of(),
        payment,
        (n) -> System.out.println("status: " + n.status())
    ),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("invoice: " + res);
    }
);
```

</section>

</custom-tabs>

**Invoice payment:** `requestInvoicePayment(mainKey, subkeys, { amount, currency, description, invoice, expires_at }, onStatusChange)` — pay an external Lightning invoice. Java: **RequestInvoicePaymentRequest**.

**Linked to subscription:** Include subscription_id in the single payment request when tying the first payment to a recurring subscription (see [Recurring Payments](recurring-payments.md)).

Handle all status values; set a timeout in your app if needed. Store preimage for proof of payment.

---

**Next:** [Recurring Payments](recurring-payments.md) · [Cashu Tokens](cashu-tokens.md) · [Profiles](profiles.md)
