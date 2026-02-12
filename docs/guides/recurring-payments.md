# Recurring Payments

Subscription-based payments with configurable billing (monthly, weekly, etc.), max payment limits, and user-controlled cancellation.

## API

**Create subscription:** `requestRecurringPayment(mainKey, subkeys, paymentRequest)` — returns `subscription_id`, `authorized_amount`, `authorized_recurrence`.

**paymentRequest:** `{ amount, currency: Currency.Millisats, recurrence, expires_at }`.  
**recurrence:** `{ calendar, first_payment_due: Timestamp, max_payments?, until?: Timestamp }`.  
**calendar:** `minutely` | `hourly` | `daily` | `weekly` | `monthly` | `quarterly` | `semiannually` | `yearly`.

```typescript
const subscription = await client.requestRecurringPayment(userPubkey, [], {
  amount: 10000,
  currency: Currency.Millisats,
  recurrence: {
    calendar: 'monthly',
    first_payment_due: Timestamp.fromNow(86400),
    max_payments: 12
  },
  expires_at: Timestamp.fromNow(3600)
});
// subscription.subscription_id, subscription.authorized_amount, etc.
```

**Listen for user cancellations:** `listenClosedRecurringPayment(onClosed)` — callback receives `{ subscription_id, main_key, reason }`; returns unsubscribe function.

**Close from provider:** `closeRecurringPayment(mainKey, subkeys, subscriptionId)`.

---

**Next:** [Profiles](profiles.md)
