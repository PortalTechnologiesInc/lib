# Recurring Payments

Subscription-based payments with configurable billing (monthly, weekly, etc.), max payment limits, and user-controlled cancellation.

## API

**Create subscription:** `requestRecurringPayment(mainKey, subkeys, paymentRequest)` — returns subscription_id, authorized_amount, authorized_recurrence.

**paymentRequest:** amount, currency (`Currency.Millisats`), recurrence, expires_at.  
**recurrence:** calendar, first_payment_due (Timestamp), max_payments, until (optional).  
**calendar:** minutely, hourly, daily, weekly, monthly, quarterly, semiannually, yearly.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.RequestRecurringPaymentRequest;
import cc.getportal.command.response.RequestRecurringPaymentResponse;
import cc.getportal.model.RecurringPaymentRequestContent;
import cc.getportal.model.RecurrenceInfo;
import cc.getportal.model.Currency;
import java.util.List;

RecurrenceInfo recurrence = new RecurrenceInfo(
    null, "monthly", null, System.currentTimeMillis() / 1000
);
RecurringPaymentRequestContent payment = new RecurringPaymentRequestContent(
    "Monthly sub", 10_000L, Currency.MILLISATS, null, recurrence,
    System.currentTimeMillis() / 1000 + 3600
);
sdk.sendCommand(
    new RequestRecurringPaymentRequest("user-pubkey-hex", List.of(), payment),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
        System.out.println("recurring: " + res);
    }
);
```

</section>

</custom-tabs>

**Listen for user cancellations:** `listenClosedRecurringPayment(onClosed)` — callback receives subscription_id, main_key, reason; returns unsubscribe function. Java: `ListenClosedRecurringPaymentRequest`.

**Close from provider:** `closeRecurringPayment(mainKey, subkeys, subscriptionId)`. Java: `CloseRecurringPaymentRequest`.

---

**Next:** [Profiles](profiles.md)
