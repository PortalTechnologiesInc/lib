# Single Payments

Accept one-time Lightning Network payments from authenticated users.

## Overview

Single payments are perfect for:
- One-time purchases
- Pay-per-use services
- Tips and donations
- Initial subscription payments
- Any transaction that happens once

## How It Works

1. User authenticates with your app
2. You request a payment with amount and description
3. Request is sent to user's Lightning wallet via Nostr
4. User approves or rejects the payment
5. You receive real-time status updates
6. Payment settles instantly on Lightning Network

## Basic Implementation

```typescript
import { PortalSDK, Currency } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

const userPubkey = 'user-public-key-hex';

await client.requestSinglePayment(
  userPubkey,
  [], // subkeys (optional)
  {
    amount: 10000,  // 10 sats (amount is in millisats)
    currency: Currency.Millisats,
    description: 'Premium subscription - 1 month'
  },
  (status) => {
    console.log('Payment status:', status.status);
    
    switch (status.status) {
      case 'paid':
        console.log('✅ Payment received!');
        console.log('Preimage:', status.preimage);
        // Grant access to service
        break;
        
      case 'user_approved':
        console.log('⏳ User approved, processing...');
        break;
        
      case 'user_rejected':
        console.log('❌ User rejected payment');
        console.log('Reason:', status.reason);
        break;
        
      case 'timeout':
        console.log('⏱️ Payment request timed out');
        break;
        
      case 'error':
        console.log('❌ Payment error:', status.reason);
        break;
    }
  }
);
```

## Payment Status Flow

```
User Receives Request
        ↓
   [user_approved]    (User approves in wallet)
        ↓
  [user_success]      (Wallet attempts payment)
        ↓
      [paid]          (Payment successful!)
```

Alternative flows:
- `user_rejected` - User explicitly declines
- `user_failed` - Payment attempt failed (insufficient funds, routing failure, etc.)
- `timeout` - User doesn't respond in time
- `error` - System error occurred

## Complete Example with Error Handling

```typescript
import { PortalSDK, Currency } from 'portal-sdk';

class PaymentService {
  private client: PortalSDK;
  
  constructor(wsUrl: string, authToken: string) {
    this.client = new PortalSDK({ serverUrl: wsUrl });
    this.init(authToken);
  }
  
  private async init(authToken: string) {
    await this.client.connect();
    await this.client.authenticate(authToken);
  }
  
  async requestPayment(
    userPubkey: string,
    amountSats: number,
    description: string
  ): Promise<{ success: boolean; preimage?: string; reason?: string }> {
    return new Promise((resolve) => {
      const timeoutMs = 60000; // 60 seconds
      const timeout = setTimeout(() => {
        resolve({
          success: false,
          reason: 'Payment request timed out'
        });
      }, timeoutMs);
      
      this.client.requestSinglePayment(
        userPubkey,
        [],
        {
          amount: amountSats * 1000, // Convert sats to millisats
          currency: Currency.Millisats,
          description
        },
        (status) => {
          if (status.status === 'paid') {
            clearTimeout(timeout);
            resolve({
              success: true,
              preimage: status.preimage
            });
          } else if (
            status.status === 'user_rejected' ||
            status.status === 'user_failed' ||
            status.status === 'error'
          ) {
            clearTimeout(timeout);
            resolve({
              success: false,
              reason: status.reason || status.status
            });
          }
          // For 'user_approved' and 'user_success', keep waiting
        }
      );
    });
  }
}

// Usage
const paymentService = new PaymentService(
  process.env.PORTAL_WS_URL!,
  process.env.PORTAL_AUTH_TOKEN!
);

const result = await paymentService.requestPayment(
  userPubkey,
  50, // 50 sats
  'Premium features access'
);

if (result.success) {
  console.log('Payment successful!');
  console.log('Proof of payment:', result.preimage);
  // Grant access to premium features
} else {
  console.log('Payment failed:', result.reason);
  // Show error message to user
}
```

## Express.js API Example

```typescript
import express from 'express';
import { PortalSDK, Currency } from 'portal-sdk';

const app = express();
app.use(express.json());

const portalClient = new PortalSDK({
  serverUrl: process.env.PORTAL_WS_URL!
});

portalClient.connect().then(() => {
  return portalClient.authenticate(process.env.PORTAL_AUTH_TOKEN!);
});

// Store pending payments
const pendingPayments = new Map<string, {
  status: string;
  preimage?: string;
  resolve: (value: any) => void;
}>();

app.post('/api/payments/create', async (req, res) => {
  const { userPubkey, amount, description } = req.body;
  
  if (!userPubkey || !amount || !description) {
    return res.status(400).json({ error: 'Missing required fields' });
  }
  
  const paymentId = generatePaymentId();
  
  // Create promise for this payment
  const paymentPromise = new Promise((resolve) => {
    pendingPayments.set(paymentId, {
      status: 'pending',
      resolve
    });
  });
  
  // Request payment
  portalClient.requestSinglePayment(
    userPubkey,
    [],
    {
      amount: amount * 1000,
      currency: Currency.Millisats,
      description
    },
    (status) => {
      const payment = pendingPayments.get(paymentId);
      if (!payment) return;
      
      if (status.status === 'paid') {
        payment.status = 'paid';
        payment.preimage = status.preimage;
        payment.resolve({ success: true, preimage: status.preimage });
        
      } else if (
        status.status === 'user_rejected' ||
        status.status === 'user_failed' ||
        status.status === 'error'
      ) {
        payment.status = 'failed';
        payment.resolve({ success: false, reason: status.reason });
      }
    }
  );
  
  res.json({ paymentId });
});

app.get('/api/payments/:paymentId/status', async (req, res) => {
  const { paymentId } = req.params;
  const payment = pendingPayments.get(paymentId);
  
  if (!payment) {
    return res.status(404).json({ error: 'Payment not found' });
  }
  
  res.json({
    status: payment.status,
    preimage: payment.preimage
  });
});

function generatePaymentId(): string {
  return `pay_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

app.listen(3001);
```

## Amount Conversion

### Sats to Millisats

```typescript
const sats = 10;
const millisats = sats * 1000;

await client.requestSinglePayment(userPubkey, [], {
  amount: millisats,
  currency: Currency.Millisats,
  description: 'Payment'
});
```

### Fiat to Sats

```typescript
// You'll need to get exchange rate from an API
async function usdToSats(usd: number): Promise<number> {
  const response = await fetch('https://api.coinbase.com/v2/exchange-rates?currency=BTC');
  const data = await response.json();
  const btcPerUsd = 1 / parseFloat(data.data.rates.USD);
  const satsPerUsd = btcPerUsd * 100000000; // 100M sats per BTC
  
  return Math.ceil(usd * satsPerUsd);
}

const usdAmount = 1.00; // $1 USD
const satsAmount = await usdToSats(usdAmount);

await client.requestSinglePayment(userPubkey, [], {
  amount: satsAmount * 1000,
  currency: Currency.Millisats,
  description: '$1.00 USD payment'
});
```

## Linking Payments to Subscriptions

You can link a single payment to a recurring subscription:

```typescript
// First, create recurring subscription
const subscription = await client.requestRecurringPayment(
  userPubkey,
  [],
  {
    amount: 10000,
    currency: Currency.Millisats,
    recurrence: {
      calendar: 'monthly',
      first_payment_due: Timestamp.fromNow(86400),
      max_payments: 12
    },
    expires_at: Timestamp.fromNow(3600)
  }
);

console.log('Subscription ID:', subscription.subscription_id);

// Then request the first payment linked to this subscription
await client.requestSinglePayment(
  userPubkey,
  [],
  {
    amount: 10000,
    currency: Currency.Millisats,
    description: 'Monthly subscription - First payment',
    subscription_id: subscription.subscription_id
  },
  (status) => {
    if (status.status === 'paid') {
      console.log('First subscription payment received!');
    }
  }
);
```

## Invoice Payments

If you have a Lightning invoice from another source, you can request the user to pay it:

```typescript
import { Timestamp } from 'portal-sdk';

await client.requestInvoicePayment(
  userPubkey,
  [],
  {
    amount: 5000,
    currency: Currency.Millisats,
    description: 'External invoice payment',
    invoice: 'lnbc50n1...', // Your Lightning invoice
    expires_at: Timestamp.fromNow(600) // 10 minutes
  },
  (status) => {
    if (status.status === 'paid') {
      console.log('Invoice paid!');
    }
  }
);
```

## Best Practices

### 1. Clear Descriptions

```typescript
// ✅ Good - Clear and specific
await client.requestSinglePayment(userPubkey, [], {
  amount: 50000,
  currency: Currency.Millisats,
  description: 'Premium Plan - 1 Month Access'
});

// ❌ Bad - Vague
await client.requestSinglePayment(userPubkey, [], {
  amount: 50000,
  currency: Currency.Millisats,
  description: 'Payment'
});
```

### 2. Handle All Status Cases

```typescript
client.requestSinglePayment(userPubkey, [], paymentRequest, (status) => {
  switch (status.status) {
    case 'paid':
      // Grant access
      break;
    case 'user_approved':
      // Show "Processing..."
      break;
    case 'user_rejected':
      // Show "Payment declined"
      break;
    case 'user_failed':
      // Show "Payment failed" + reason
      break;
    case 'timeout':
      // Show "Request expired"
      break;
    case 'error':
      // Log error, show generic message
      break;
  }
});
```

### 3. Store Payment Proofs

```typescript
const payments = new Map<string, {
  userPubkey: string;
  amount: number;
  description: string;
  preimage: string;
  timestamp: number;
}>();

client.requestSinglePayment(userPubkey, [], request, (status) => {
  if (status.status === 'paid') {
    payments.set(generatePaymentId(), {
      userPubkey,
      amount: request.amount,
      description: request.description,
      preimage: status.preimage!,
      timestamp: Date.now()
    });
  }
});
```

### 4. Set Reasonable Timeouts

```typescript
// Don't wait forever
const MAX_WAIT = 120000; // 2 minutes

const timeout = setTimeout(() => {
  console.log('Payment request expired');
  // Notify user
}, MAX_WAIT);

client.requestSinglePayment(userPubkey, [], request, (status) => {
  if (status.status === 'paid' || 
      status.status === 'user_rejected' ||
      status.status === 'user_failed') {
    clearTimeout(timeout);
  }
});
```

### 5. Validate Amounts

```typescript
function validatePaymentAmount(sats: number): boolean {
  const MIN_SATS = 1;
  const MAX_SATS = 1000000; // 0.01 BTC
  
  return sats >= MIN_SATS && sats <= MAX_SATS;
}

if (!validatePaymentAmount(amount)) {
  throw new Error('Invalid payment amount');
}
```

## Troubleshooting

### Payment Never Completes

**Possible causes:**
- User's wallet is offline
- Network connectivity issues
- Lightning routing failures
- Insufficient channel capacity

**Solutions:**
- Implement reasonable timeouts
- Show status to user ("Waiting for payment...")
- Allow users to retry
- Provide alternative payment methods

### "User Rejected" Status

**Causes:**
- User explicitly declined
- Amount too high
- Insufficient funds
- User doesn't trust the request

**Solutions:**
- Show clear description of what they're paying for
- Display amount in both sats and fiat
- Build trust with clear branding
- Allow users to try again

### Routing Failures

**Causes:**
- Recipient node unreachable
- No route with sufficient capacity
- Channel liquidity issues

**Solutions:**
- Ensure your NWC wallet has good connectivity
- Use a well-connected Lightning node
- Consider using a hosted Lightning service
- Set up multiple channels

---

**Next Steps**:
- [Recurring Payments](recurring-payments.md) - Set up subscriptions
- [Cashu Tokens](cashu-tokens.md) - Issue tickets and vouchers
- [Profile Management](profiles.md) - Fetch user information

