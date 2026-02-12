# Single Payments

One-time Lightning payments from an authenticated user. You request a payment; the user approves or rejects in their wallet; you get status updates and, on success, a preimage.

## API

`requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange)`

- **paymentRequest:** `{ amount, currency: Currency.Millisats, description }` — amount in millisats (1 sat = 1000).
- **onStatusChange:** callback receives status objects. `status.status`: `paid` | `user_approved` | `user_rejected` | `user_failed` | `timeout` | `error`. On `paid`, use `status.preimage`; on failure, `status.reason`.

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

**Invoice payment:** `requestInvoicePayment(mainKey, subkeys, { amount, currency, description, invoice, expires_at }, onStatusChange)` — pay an external Lightning invoice.

**Linked to subscription:** Include `subscription_id` in the single payment request when tying the first payment to a recurring subscription (see [Recurring Payments](recurring-payments.md)).

Handle all status values; set a timeout in your app if needed. Store preimage for proof of payment.

---

**Next:** [Recurring Payments](recurring-payments.md) · [Cashu Tokens](cashu-tokens.md) · [Profiles](profiles.md)
