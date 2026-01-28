# Cashu Tokens (Tickets)

Use Cashu ecash tokens as tickets, vouchers, or transferable access tokens in your application.

## What is Cashu?

Cashu is an ecash protocol built on Bitcoin's Lightning Network. It provides:

- **Privacy**: Tokens are untraceable bearer instruments
- **Offline Transfers**: Can be sent peer-to-peer without internet
- **Interoperability**: Works across different applications
- **Bitcoin-Backed**: Each token is backed by real sats
- **Blind Signatures**: Mint cannot track token usage

## Use Cases

### Event Tickets
Issue tokens that grant access to events. Users present the token at entry, and you burn it to verify authenticity.

### Vouchers & Gift Cards
Create tokens worth a specific amount that users can redeem for products or services.

### Access Tokens
Grant temporary or permanent access to premium features using tokens.

### Transferable Subscriptions
Allow users to share or resell access by transferring tokens.

## How It Works

1. **Mint Tokens**: Create Cashu tokens backed by sats
2. **Send to Users**: Transfer tokens to authenticated users
3. **Users Hold Tokens**: Users store tokens in their Cashu-compatible wallet
4. **Redeem/Burn**: Verify and burn tokens when user accesses service

## Requesting Cashu Tokens from Users

Ask users to send you Cashu tokens (e.g., as payment or ticket redemption):

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

const userPubkey = 'user-public-key-hex';

// Request Cashu tokens from user
const result = await client.requestCashu(
  userPubkey,
  [], // subkeys
  'https://mint.example.com', // mint URL
  'sat', // unit (usually 'sat')
  10000 // amount in millisats (10 sats)
);

if (result.status === 'success') {
  console.log('Received Cashu token:', result.token);
  
  // Now burn the token to claim the sats
  const amount = await client.burnCashu(
    'https://mint.example.com',
    'sat',
    result.token,
    undefined // static_token (optional, for private mints)
  );
  
  console.log('Claimed amount:', amount);
} else if (result.status === 'insufficient_funds') {
  console.log('User has insufficient funds');
} else if (result.status === 'rejected') {
  console.log('User rejected the request');
}
```

## Sending Cashu Tokens to Users

Send tokens directly to authenticated users:

```typescript
// First, mint a token
const cashuToken = await client.mintCashu(
  'https://mint.example.com', // mint URL
  undefined, // static auth token (if mint requires it)
  'sat', // unit
  10000, // amount in millisats (10 sats)
  'Event Ticket - VIP Access' // description
);

console.log('Minted token:', cashuToken);

// Send token to user
const userPubkey = 'user-public-key-hex';

const message = await client.sendCashuDirect(
  userPubkey,
  [], // subkeys
  cashuToken
);

console.log('Sent to user:', message);
```

## Complete Ticket Issuance Flow

Here's a complete example of issuing event tickets:

```typescript
import { PortalSDK, Currency } from 'portal-sdk';

class TicketSystem {
  private client: PortalSDK;
  private mintUrl = 'https://mint.example.com';
  
  constructor(wsUrl: string, authToken: string) {
    this.client = new PortalSDK({ serverUrl: wsUrl });
    this.init(authToken);
  }

  private async init(authToken: string) {
    await this.client.connect();
    await this.client.authenticate(authToken);
  }

  async issueTicket(userPubkey: string, ticketType: string, price: number): Promise<boolean> {
    try {
      // 1. Request payment from user
      const paymentReceived = await this.requestPayment(userPubkey, price);
      
      if (!paymentReceived) {
        console.log('Payment failed or rejected');
        return false;
      }

      // 2. Mint a Cashu token as the ticket
      const ticket = await this.client.mintCashu(
        this.mintUrl,
        undefined,
        'sat',
        price,
        `Ticket: ${ticketType}`
      );

      // 3. Send ticket to user
      await this.client.sendCashuDirect(userPubkey, [], ticket);
      
      console.log('Ticket issued successfully!');
      return true;
      
    } catch (error) {
      console.error('Failed to issue ticket:', error);
      return false;
    }
  }

  private async requestPayment(userPubkey: string, amount: number): Promise<boolean> {
    return new Promise((resolve) => {
      this.client.requestSinglePayment(
        userPubkey,
        [],
        {
          amount,
          currency: Currency.Millisats,
          description: 'Event ticket purchase'
        },
        (status) => {
          if (status.status === 'paid') {
            resolve(true);
          } else if (status.status === 'user_rejected' || status.status === 'timeout') {
            resolve(false);
          }
        }
      );
    });
  }

  async verifyAndRedeemTicket(userPubkey: string): Promise<boolean> {
    try {
      // Request the ticket back from user
      const result = await this.client.requestCashu(
        userPubkey,
        [],
        this.mintUrl,
        'sat',
        1000 // ticket value
      );

      if (result.status === 'success') {
        // Burn the token to verify it and prevent reuse
        const amount = await this.client.burnCashu(
          this.mintUrl,
          'sat',
          result.token
        );
        
        console.log('Ticket verified and redeemed!');
        return true;
      } else {
        console.log('Invalid or already used ticket');
        return false;
      }
    } catch (error) {
      console.error('Ticket verification failed:', error);
      return false;
    }
  }
}

// Usage
const ticketSystem = new TicketSystem(
  process.env.PORTAL_WS_URL!,
  process.env.PORTAL_AUTH_TOKEN!
);

// Issue a VIP ticket
await ticketSystem.issueTicket(
  userPubkey,
  'VIP Access',
  50000 // 50 sats
);

// Later, when user arrives at event
await ticketSystem.verifyAndRedeemTicket(userPubkey);
```

## Voucher System Example

Create a gift voucher system:

```typescript
class VoucherSystem {
  private client: PortalSDK;
  private mintUrl = 'https://mint.example.com';

  async createVoucher(value: number, description: string): Promise<string> {
    // Mint a token worth the voucher value
    const voucher = await this.client.mintCashu(
      this.mintUrl,
      undefined,
      'sat',
      value,
      `Voucher: ${description}`
    );

    return voucher;
  }

  async sendVoucher(recipientPubkey: string, voucher: string) {
    await this.client.sendCashuDirect(recipientPubkey, [], voucher);
    console.log('Voucher sent to recipient');
  }

  async redeemVoucher(userPubkey: string, voucherValue: number): Promise<number> {
    const result = await this.client.requestCashu(
      userPubkey,
      [],
      this.mintUrl,
      'sat',
      voucherValue
    );

    if (result.status === 'success') {
      // Burn and claim the value
      const amount = await this.client.burnCashu(
        this.mintUrl,
        'sat',
        result.token
      );

      return amount;
    }

    return 0;
  }
}

// Create and send a $5 voucher (assuming 1 sat = $0.0001)
const voucher = await voucherSystem.createVoucher(50000, '$5 Store Credit');
await voucherSystem.sendVoucher(recipientPubkey, voucher);

// Redeem voucher
const value = await voucherSystem.redeemVoucher(userPubkey, 50000);
console.log('Voucher redeemed for:', value, 'millisats');
```

## API Reference

### mintCashu()

Mint new Cashu tokens from a mint:

```typescript
await client.mintCashu(
  mintUrl: string,      // Mint URL
  staticAuthToken?: string,  // Optional: auth token for private mints
  unit: string,         // Usually 'sat'
  amount: number,       // Amount in millisats
  description?: string  // Optional description
): Promise<string>
```

Returns: Cashu token as a string

### burnCashu()

Burn (redeem) a Cashu token at a mint:

```typescript
await client.burnCashu(
  mintUrl: string,           // Mint URL
  unit: string,              // Usually 'sat'
  token: string,             // Cashu token to burn
  staticAuthToken?: string   // Optional: auth token for private mints
): Promise<number>
```

Returns: Amount claimed in millisats

### requestCashu()

Request Cashu tokens from a user:

```typescript
await client.requestCashu(
  recipientKey: string,  // User's public key
  subkeys: string[],     // Optional subkeys
  mintUrl: string,       // Mint URL
  unit: string,          // Usually 'sat'
  amount: number         // Amount to request
): Promise<CashuResponseStatus>
```

Returns:
```typescript
{
  status: 'success' | 'insufficient_funds' | 'rejected',
  token?: string  // If status is 'success'
  reason?: string // If status is 'rejected'
}
```

### sendCashuDirect()

Send Cashu tokens directly to a user:

```typescript
await client.sendCashuDirect(
  mainKey: string,    // User's public key
  subkeys: string[],  // Optional subkeys
  token: string       // Cashu token to send
): Promise<string>
```

Returns: Success message

## Setting Up Your Own Mint

For complete control over token issuance and custom ticket types, run your own Cashu mint using Portal's enhanced CDK implementation.

**Full Guide**: See [Running a Custom Mint](running-a-mint.md) for detailed instructions on:
- Docker deployment with `getportal/cdk-mintd`
- Creating custom units (VIP, General, etc.)
- Adding metadata and images to tokens
- Event ticket configuration
- Authentication and security

**Quick Start**:
```bash
docker pull getportal/cdk-mintd:latest
# Configure and run - see full guide for details
```

### Public Mints

If you don't want to run your own mint, you can use public Cashu mints:
- https://mint.minibits.cash
- https://mint.bitcoinmints.com
- https://stablenut.umint.cash

To find more mints, check: https://bitcoinmints.com/

## Security Considerations

### 1. Token Storage

Cashu tokens are bearer instruments. Anyone with the token can spend it:

```typescript
// ❌ Don't log tokens in production
console.log('Token:', cashuToken);

// ❌ Don't store in plain text
fs.writeFileSync('token.txt', cashuToken);

// ✅ Handle securely
// Only send directly to users, don't store
```

### 2. Double-Spending Prevention

Always burn tokens immediately after receiving:

```typescript
const result = await client.requestCashu(user, [], mintUrl, 'sat', 1000);

if (result.status === 'success') {
  // Burn immediately to prevent reuse
  await client.burnCashu(mintUrl, 'sat', result.token);
}
```

### 3. Mint Trust

You must trust the mint operator:
- Use reputable, well-known mints for production
- Consider running your own mint for full control
- Diversify across multiple mints for redundancy

### 4. Amount Validation

Always validate amounts before minting:

```typescript
function validateAmount(amount: number): boolean {
  return amount > 0 && amount <= 100000000; // Max 100k sats
}

if (validateAmount(ticketPrice)) {
  await client.mintCashu(mintUrl, undefined, 'sat', ticketPrice);
}
```

## Troubleshooting

### "Insufficient funds" Error

User's wallet doesn't have enough Cashu tokens at that mint:
- They may need to mint tokens first
- They may have tokens at a different mint

### Mint Connection Issues

```typescript
try {
  await client.mintCashu(mintUrl, undefined, 'sat', 1000);
} catch (error) {
  console.error('Mint error:', error);
  // Try alternative mint or notify user
}
```

### Token Already Spent

If burning fails, the token may have already been redeemed:
- Tokens can only be spent once
- Implement proper tracking to prevent this

---

**Next Steps**:
- [JWT Tokens](jwt-tokens.md) - Session management with JWTs
- [Single Payments](single-payments.md) - Accept Lightning payments
- [Docker Deployment](../getting-started/docker-deployment.md) - Deploy securely

