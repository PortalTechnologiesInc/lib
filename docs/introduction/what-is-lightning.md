# What is Lightning Network?

The Lightning Network is a "Layer 2" payment protocol built on top of Bitcoin that enables fast, low-cost transactions.

## Why Lightning?

Traditional Bitcoin transactions:
- Can take 10+ minutes to confirm
- Have transaction fees that can be high during busy times
- Are recorded on the blockchain forever

Lightning Network transactions:
- Are **instant** (sub-second)
- Have **minimal fees** (often less than 1 satoshi)
- Are **private** (not all details go on the blockchain)
- Enable **micropayments** (pay fractions of a cent)

## How It Works (Simplified)

1. **Payment Channels**: Two parties open a channel by creating a special Bitcoin transaction
2. **Off-Chain Transactions**: They can then make unlimited instant transactions between each other
3. **Network of Channels**: Payments can route through multiple channels to reach any destination
4. **Settlement**: Channels can be closed at any time, settling the final balance on the Bitcoin blockchain

## Lightning in Portal

Portal uses Lightning Network for:

### Single Payments
One-time payments for purchases, tips, or services:
```typescript
await client.requestSinglePayment(
  userKey,
  [],
  {
    amount: 1000,        // 1 sat (1000 millisats)
    currency: Currency.Millisats,
    description: "Premium subscription"
  },
  (status) => {
    if (status.status === 'paid') {
      console.log('Payment received!');
    }
  }
);
```

### Recurring Payments
Subscription-based payments with automatic billing:
```typescript
await client.requestRecurringPayment(
  userKey,
  [],
  {
    amount: 10000,       // 10 sats per month
    currency: Currency.Millisats,
    recurrence: {
      calendar: "monthly",
      first_payment_due: Timestamp.fromNow(86400),
      max_payments: 12
    },
    expires_at: Timestamp.fromNow(3600)
  }
);
```

## Nostr Wallet Connect (NWC)

Portal uses **Nostr Wallet Connect**, a protocol that allows:

- Requesting payments through Nostr messages
- User approval through their Lightning wallet
- Real-time payment status updates
- Non-custodial payment flow (users maintain control of funds)

The user's Lightning wallet could be:
- [Alby](https://getalby.com/)
- [Mutiny](https://www.mutinywallet.com/)
- [Breez](https://breez.technology/)
- Any other NWC-compatible wallet

## Payment Flow in Portal

1. **Payment Request**: Your app requests a payment through Portal
2. **Nostr Message**: Request is sent to the user via Nostr
3. **Wallet Notification**: User's wallet shows the payment request
4. **User Approval**: User approves or denies the payment
5. **Lightning Payment**: Wallet sends payment via Lightning Network
6. **Confirmation**: Your app receives real-time payment confirmation

All within seconds, with minimal fees.

## Benefits for Your Business

- **Instant Settlement**: Receive payments immediately
- **Global Reach**: Accept payments from anyone, anywhere
- **No Chargebacks**: Bitcoin payments are final
- **Low Fees**: Typically < 1% (often much less)
- **No Middlemen**: Direct payment from customer to you
- **Privacy**: No personal information required

## Learn More

- [Lightning Network Overview](https://lightning.network/)
- [Understanding Lightning Network](https://bitcoin.org/en/bitcoin-core/capacity-increases-faq#lightning-network)
- [Nostr Wallet Connect (NWC)](https://nwc.getalby.com/)

---

**Next**: Start integrating Portal with the [Quick Start Guide](../getting-started/quick-start.md)

