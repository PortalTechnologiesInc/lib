# Recurring Payments

Set up subscription-based payments with customizable billing cycles.

## Overview

Recurring payments enable subscription business models with:
- Automatic billing on custom schedules
- Monthly, weekly, or custom recurrence patterns
- Maximum payment limits
- User-controlled subscription management

## Basic Implementation

```typescript
import { PortalSDK, Currency, Timestamp } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

const userPubkey = 'user-public-key-hex';

const subscription = await client.requestRecurringPayment(
  userPubkey,
  [], // subkeys
  {
    amount: 10000, // 10 sats per payment
    currency: Currency.Millisats,
    recurrence: {
      calendar: 'monthly', // or 'weekly', 'daily', etc.
      first_payment_due: Timestamp.fromNow(86400), // 24 hours from now
      max_payments: 12, // Optional: limit total payments
      until: Timestamp.fromDate(new Date('2025-12-31')) // Optional: end date
    },
    expires_at: Timestamp.fromNow(3600) // Request expires in 1 hour
  }
);

console.log('Subscription created!');
console.log('Subscription ID:', subscription.subscription_id);
console.log('Authorized amount:', subscription.authorized_amount);
console.log('Recurrence:', subscription.authorized_recurrence);
```

## Recurrence Patterns

Portal supports the following calendar frequencies:
- `minutely` - Every minute (for testing)
- `hourly` - Every hour
- `daily` - Every day
- `weekly` - Every week
- `monthly` - Every month
- `quarterly` - Every 3 months
- `semiannually` - Every 6 months
- `yearly` - Every year

### Monthly Subscription

```typescript
{
  calendar: 'monthly',
  first_payment_due: Timestamp.fromNow(86400), // Start tomorrow
  max_payments: 12 // 1 year
}
```

### Weekly Subscription

```typescript
{
  calendar: 'weekly',
  first_payment_due: Timestamp.fromNow(604800), // Start next week
  max_payments: 52 // 1 year
}
```

### Daily Subscription

```typescript
{
  calendar: 'daily',
  first_payment_due: Timestamp.fromNow(86400), // Start tomorrow
  max_payments: 30 // 30 days
}
```

## Listening for Subscription Closures

Users can cancel subscriptions at any time. Listen for these events:

```typescript
await client.listenClosedRecurringPayment((data) => {
  console.log('Subscription closed!');
  console.log('Subscription ID:', data.subscription_id);
  console.log('User:', data.main_key);
  console.log('Reason:', data.reason);
  
  // Revoke access for this user
  removeUserAccess(data.main_key);
});
```

## Closing Subscriptions (Provider Side)

You can also close subscriptions from your side:

```typescript
const message = await client.closeRecurringPayment(
  userPubkey,
  [],
  subscriptionId
);

console.log(message); // "Subscription closed successfully"
```

## Complete Subscription Service Example

```typescript
class SubscriptionService {
  private client: PortalSDK;
  private subscriptions = new Map<string, {
    userPubkey: string;
    subscriptionId: string;
    amount: number;
    status: 'active' | 'cancelled';
  }>();

  constructor(wsUrl: string, authToken: string) {
    this.client = new PortalSDK({ serverUrl: wsUrl });
    this.init(authToken);
  }

  private async init(authToken: string) {
    await this.client.connect();
    await this.client.authenticate(authToken);
    
    // Listen for user cancellations
    await this.client.listenClosedRecurringPayment((data) => {
      this.handleCancellation(data);
    });
  }

  async createSubscription(
    userPubkey: string,
    plan: 'basic' | 'premium'
  ): Promise<string> {
    const plans = {
      basic: { amount: 10000, name: 'Basic Plan' },
      premium: { amount: 50000, name: 'Premium Plan' }
    };

    const selectedPlan = plans[plan];

    const result = await this.client.requestRecurringPayment(
      userPubkey,
      [],
      {
        amount: selectedPlan.amount,
        currency: Currency.Millisats,
        recurrence: {
          calendar: 'monthly',
          first_payment_due: Timestamp.fromNow(86400),
          max_payments: 12
        },
        expires_at: Timestamp.fromNow(3600)
      }
    );

    const subscriptionId = result.subscription_id;

    // Store subscription
    this.subscriptions.set(subscriptionId, {
      userPubkey,
      subscriptionId,
      amount: selectedPlan.amount,
      status: 'active'
    });

    return subscriptionId;
  }

  async cancelSubscription(subscriptionId: string) {
    const sub = this.subscriptions.get(subscriptionId);
    if (!sub) throw new Error('Subscription not found');

    await this.client.closeRecurringPayment(
      sub.userPubkey,
      [],
      subscriptionId
    );

    sub.status = 'cancelled';
  }

  private handleCancellation(data: any) {
    const sub = this.subscriptions.get(data.subscription_id);
    if (sub) {
      sub.status = 'cancelled';
      console.log(`User ${sub.userPubkey} cancelled subscription`);
      
      // Revoke access, send notification, etc.
    }
  }
}
```

---

**Next**: [Profile Management](profiles.md)

