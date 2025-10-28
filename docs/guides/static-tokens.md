# Static Tokens & Physical Authentication

Use static tokens to create reusable authentication URLs for physical locations, enabling both online and in-person use cases.

## What are Static Tokens?

Static tokens are unique identifiers you can embed in authentication URLs to create persistent, location-specific authentication points. Unlike regular authentication URLs that are single-use, static token URLs can be:

- **Printed as QR codes** on physical materials
- **Written to NFC stickers** for contactless authentication
- **Reused indefinitely** without regeneration
- **Location-specific** to track where requests originate

## Why Use Static Tokens?

Static tokens enable Portal to work in the **physical world**, not just online:

✅ **Restaurant Tables** - Print QR codes on tables for payment requests
✅ **Office Access** - NFC stickers on doors for authentication
✅ **Event Check-in** - Unique codes per entrance for tracking
✅ **Vending Machines** - Physical payment endpoints
✅ **Hotel Rooms** - Room-specific authentication for services
✅ **Retail Checkout** - Counter-specific payment requests

## How It Works

### 1. Generate URL with Static Token

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

// Create reusable URL with static token
const staticToken = 'table-14-restaurant-a';

const authUrl = await client.newKeyHandshakeUrl(
  (mainKey, preferredRelays) => {
    console.log(`Authentication from: ${staticToken}`);
    console.log(`User: ${mainKey}`);
    
    // Handle based on location
    handleLocationAuth(staticToken, mainKey);
  },
  staticToken // Static token parameter
);

console.log('Reusable URL:', authUrl);
// This URL can be used multiple times!
```

### 2. The URL is Reusable

Unlike regular authentication URLs that expire after one use, static token URLs can be:
- Scanned multiple times
- Printed and distributed
- Embedded in physical objects
- Used by different users

### 3. Track Request Origin

```typescript
function handleLocationAuth(location: string, userPubkey: string) {
  // Parse location from static token
  const [type, id, venue] = location.split('-');
  
  switch (type) {
    case 'table':
      console.log(`User at table ${id} in ${venue}`);
      // Send menu, track orders by table
      break;
      
    case 'door':
      console.log(`Access request at ${id}`);
      // Check permissions, unlock door
      break;
      
    case 'kiosk':
      console.log(`Kiosk ${id} authentication`);
      // Load user preferences
      break;
  }
}
```

## Use Case: Restaurant Tables

### Setup

```typescript
class RestaurantService {
  private client: PortalSDK;
  private tableUrls = new Map<number, string>();
  
  async generateTableQRCodes(tableCount: number) {
    const urls: Array<{ table: number; url: string }> = [];
    
    for (let tableNum = 1; tableNum <= tableCount; tableNum++) {
      const staticToken = `table-${tableNum}-myrestaurant`;
      
      const authUrl = await this.client.newKeyHandshakeUrl(
        async (mainKey) => {
          console.log(`Table ${tableNum}: User ${mainKey} authenticated`);
          
          // Authenticate user
          const authResponse = await this.client.authenticateKey(mainKey);
          
          if (authResponse.status.status === 'approved') {
            // Associate user with table
            this.assignUserToTable(tableNum, mainKey);
            
            // Send digital menu
            this.sendMenu(mainKey);
          }
        },
        staticToken
      );
      
      this.tableUrls.set(tableNum, authUrl);
      urls.push({ table: tableNum, url: authUrl });
    }
    
    return urls;
  }
  
  async requestTablePayment(tableNum: number, amount: number) {
    const userPubkey = this.getTableUser(tableNum);
    
    if (!userPubkey) {
      throw new Error('No user at this table');
    }
    
    return new Promise((resolve) => {
      this.client.requestSinglePayment(
        userPubkey,
        [],
        {
          amount: amount * 1000,
          currency: Currency.Millisats,
          description: `Payment for Table ${tableNum}`
        },
        (status) => {
          if (status.status === 'paid') {
            console.log(`Table ${tableNum} paid!`);
            this.clearTable(tableNum);
            resolve(true);
          }
        }
      );
    });
  }
  
  private assignUserToTable(table: number, pubkey: string) {
    // Implementation...
  }
  
  private getTableUser(table: number): string | null {
    // Implementation...
    return null;
  }
  
  private clearTable(table: number) {
    // Implementation...
  }
  
  private sendMenu(pubkey: string) {
    // Send menu items via Nostr direct messages
  }
}

// Usage
const restaurant = new RestaurantService();

// Generate QR codes for 20 tables
const qrCodes = await restaurant.generateTableQRCodes(20);

// Print QR codes
for (const { table, url } of qrCodes) {
  console.log(`Table ${table}:`);
  await generateQRCodeImage(url, `table-${table}.png`);
}

// Later, when bill is ready
await restaurant.requestTablePayment(14, 45); // Table 14, 45 sats
```

### Generating QR Codes

```typescript
import QRCode from 'qrcode';
import fs from 'fs';

async function generateQRCodeImage(url: string, filename: string) {
  // Generate PNG
  await QRCode.toFile(filename, url, {
    width: 400,
    margin: 2,
    color: {
      dark: '#000000',
      light: '#FFFFFF'
    }
  });
  
  console.log(`QR code saved: ${filename}`);
}

async function generateQRCodeSVG(url: string, filename: string) {
  // Generate SVG for print quality
  const svg = await QRCode.toString(url, { type: 'svg' });
  fs.writeFileSync(filename, svg);
  
  console.log(`QR code saved: ${filename}`);
}

// Generate for all tables
const tableUrl = await client.newKeyHandshakeUrl(handler, 'table-1');
await generateQRCodeImage(tableUrl, 'table-1-qr.png');
await generateQRCodeSVG(tableUrl, 'table-1-qr.svg'); // For printing
```

## Use Case: Office Door Access

```typescript
class DoorAccessSystem {
  private client: PortalSDK;
  private authorizedUsers = new Set<string>();
  
  async setupDoor(doorId: string) {
    const staticToken = `door-${doorId}`;
    
    const nfcUrl = await this.client.newKeyHandshakeUrl(
      async (mainKey) => {
        console.log(`Access attempt at ${doorId} by ${mainKey}`);
        
        // Authenticate user
        const authResponse = await this.client.authenticateKey(mainKey);
        
        if (authResponse.status.status === 'approved') {
          // Check if user has access
          if (this.authorizedUsers.has(mainKey)) {
            console.log('✅ Access granted');
            this.unlockDoor(doorId);
            this.logAccess(doorId, mainKey, 'granted');
          } else {
            console.log('❌ Access denied - not authorized');
            this.logAccess(doorId, mainKey, 'denied');
          }
        }
      },
      staticToken
    );
    
    console.log(`Write this URL to NFC sticker for ${doorId}:`);
    console.log(nfcUrl);
    
    return nfcUrl;
  }
  
  addAuthorizedUser(pubkey: string) {
    this.authorizedUsers.add(pubkey);
  }
  
  removeAuthorizedUser(pubkey: string) {
    this.authorizedUsers.delete(pubkey);
  }
  
  private unlockDoor(doorId: string) {
    // Send signal to smart lock
    console.log(`Door ${doorId} unlocked for 5 seconds`);
  }
  
  private logAccess(doorId: string, user: string, result: 'granted' | 'denied') {
    const log = {
      timestamp: new Date(),
      door: doorId,
      user,
      result
    };
    // Store in database
    console.log('Access log:', log);
  }
}

// Setup
const doorSystem = new DoorAccessSystem();

// Generate NFC URLs for different doors
await doorSystem.setupDoor('main-entrance');
await doorSystem.setupDoor('server-room');
await doorSystem.setupDoor('executive-office');

// Authorize users
doorSystem.addAuthorizedUser('user-pubkey-1'); // Main entrance
doorSystem.addAuthorizedUser('user-pubkey-2'); // All doors
```

## Use Case: Event Entrances

Track which entrance each guest uses:

```typescript
class EventCheckIn {
  private client: PortalSDK;
  private checkedInGuests = new Map<string, string>(); // pubkey -> entrance
  
  async setupEntrances(entrances: string[]) {
    const urls: Map<string, string> = new Map();
    
    for (const entrance of entrances) {
      const staticToken = `entrance-${entrance}`;
      
      const url = await this.client.newKeyHandshakeUrl(
        async (mainKey) => {
          console.log(`Guest ${mainKey} at ${entrance}`);
          
          const authResponse = await this.client.authenticateKey(mainKey);
          
          if (authResponse.status.status === 'approved') {
            // Check in guest
            this.checkedInGuests.set(mainKey, entrance);
            
            // Request ticket payment if needed
            await this.verifyTicket(mainKey);
            
            console.log(`✅ Checked in at ${entrance}`);
          }
        },
        staticToken
      );
      
      urls.set(entrance, url);
    }
    
    return urls;
  }
  
  async verifyTicket(userPubkey: string) {
    // Request Cashu ticket token
    const result = await this.client.requestCashu(
      userPubkey,
      [],
      'https://mint.example.com',
      'vip',
      1
    );
    
    if (result.status === 'success') {
      // Burn to verify
      await this.client.burnCashu(
        'https://mint.example.com',
        'vip',
        result.token
      );
      return true;
    }
    
    return false;
  }
  
  getEntranceStats() {
    const stats = new Map<string, number>();
    for (const entrance of this.checkedInGuests.values()) {
      stats.set(entrance, (stats.get(entrance) || 0) + 1);
    }
    return stats;
  }
}

// Usage
const event = new EventCheckIn();

const entranceUrls = await event.setupEntrances([
  'main-entrance',
  'vip-entrance',
  'backstage'
]);

// Print QR codes for each entrance
for (const [entrance, url] of entranceUrls) {
  await generateQRCodeImage(url, `${entrance}-checkin.png`);
}

// Later: view stats
console.log('Check-in stats:', event.getEntranceStats());
// { 'main-entrance': 145, 'vip-entrance': 23, 'backstage': 8 }
```

## NFC Integration Concepts

While Portal SDK doesn't directly handle NFC hardware, you can integrate with NFC-capable apps:

### Writing to NFC

1. Generate URL with static token
2. Use NFC writing app to write the URL as an NDEF record
3. Place sticker at physical location

### Reading from NFC (Mobile App)

When a user's Nostr-compatible wallet app supports NFC:

1. User taps phone on NFC sticker
2. App reads the Portal authentication URL
3. App opens the authentication flow
4. User approves authentication
5. Your backend receives the callback with the static token
6. You know which physical location they're at

### Example NFC Data Format

```
NDEF Record:
Type: URI
Data: nostr:nprofile1[...]static-token=table-5
```

## Location-Based Routing

Use static tokens to route requests differently:

```typescript
const locationHandlers = {
  'table-': (token: string, user: string) => {
    const tableNum = token.split('-')[1];
    return handleRestaurantTable(tableNum, user);
  },
  
  'door-': (token: string, user: string) => {
    const doorId = token.split('-')[1];
    return handleDoorAccess(doorId, user);
  },
  
  'kiosk-': (token: string, user: string) => {
    const kioskId = token.split('-')[1];
    return handleKioskAuth(kioskId, user);
  }
};

async function handleStaticTokenAuth(staticToken: string, userPubkey: string) {
  // Find handler based on token prefix
  for (const [prefix, handler] of Object.entries(locationHandlers)) {
    if (staticToken.startsWith(prefix)) {
      return handler(staticToken, userPubkey);
    }
  }
  
  // Default handler
  return handleGenericAuth(userPubkey);
}

// Generate URLs with routing
const tableUrl = await client.newKeyHandshakeUrl(
  (mainKey) => handleStaticTokenAuth('table-5', mainKey),
  'table-5'
);

const doorUrl = await client.newKeyHandshakeUrl(
  (mainKey) => handleStaticTokenAuth('door-main', mainKey),
  'door-main'
);
```

## Security Considerations

### 1. Static Token Entropy

Use sufficiently random static tokens:

```typescript
import crypto from 'crypto';

function generateStaticToken(prefix: string): string {
  const random = crypto.randomBytes(16).toString('hex');
  return `${prefix}-${random}`;
}

// Good: table-5-a3f9d2e1c4b8...
const token = generateStaticToken('table-5');
```

### 2. Token Rotation

Periodically rotate static tokens for sensitive locations:

```typescript
class TokenManager {
  private activeTokens = new Map<string, Date>();
  
  async rotateToken(location: string, oldToken: string) {
    const newToken = generateStaticToken(location);
    
    // Generate new URL
    const newUrl = await client.newKeyHandshakeUrl(
      handler,
      newToken
    );
    
    // Mark old token as deprecated
    this.activeTokens.set(newToken, new Date());
    
    // Give grace period before removing old
    setTimeout(() => {
      this.activeTokens.delete(oldToken);
    }, 86400000); // 24 hours
    
    return { token: newToken, url: newUrl };
  }
}
```

### 3. Physical Security

- **QR Codes**: Consider using tamper-evident materials
- **NFC Stickers**: Use stickers with tamper detection
- **Location**: Place in supervised areas when possible
- **Monitoring**: Log all authentication attempts with timestamps

### 4. Access Control

Verify user permissions based on location:

```typescript
const permissions = {
  'table-1': ['menu', 'order', 'payment'],
  'door-serverroom': ['authenticated-staff-only'],
  'kiosk-lobby': ['check-in', 'directions']
};

async function checkPermission(staticToken: string, userPubkey: string, action: string) {
  const requiredPerms = permissions[staticToken] || [];
  const userPerms = await getUserPermissions(userPubkey);
  
  return requiredPerms.some(perm => userPerms.includes(perm));
}
```

## Analytics & Insights

Track physical location usage:

```typescript
class LocationAnalytics {
  private events: Array<{
    timestamp: Date;
    location: string;
    user: string;
    action: string;
  }> = [];
  
  logEvent(location: string, user: string, action: string) {
    this.events.push({
      timestamp: new Date(),
      location,
      user,
      action
    });
  }
  
  getLocationStats(timeframe: 'hour' | 'day' | 'week') {
    // Aggregate by location
    const stats = new Map<string, number>();
    
    for (const event of this.events) {
      stats.set(event.location, (stats.get(event.location) || 0) + 1);
    }
    
    return stats;
  }
  
  getPeakTimes(location: string) {
    const hourCounts = new Array(24).fill(0);
    
    for (const event of this.events) {
      if (event.location === location) {
        const hour = event.timestamp.getHours();
        hourCounts[hour]++;
      }
    }
    
    return hourCounts;
  }
}

// Usage
const analytics = new LocationAnalytics();

// In your handler
const url = await client.newKeyHandshakeUrl(
  (mainKey) => {
    analytics.logEvent('table-5', mainKey, 'authenticated');
    // ...rest of handler
  },
  'table-5'
);

// Later: analyze
console.log('Busiest tables:', analytics.getLocationStats('day'));
console.log('Table 5 peak hours:', analytics.getPeakTimes('table-5'));
```

## Best Practices

### 1. Naming Conventions

Use consistent token naming:

```typescript
// Good patterns:
'table-{number}-{venue}'      // table-14-downtown
'door-{building}-{room}'      // door-hq-serverroom  
'kiosk-{location}-{number}'   // kiosk-lobby-1
'entrance-{event}-{gate}'     // entrance-concert-a
```

### 2. QR Code Printing

For physical QR codes:

```typescript
await QRCode.toFile('table-5.png', url, {
  width: 600,           // Large enough to scan easily
  margin: 4,            // White border for reliability
  errorCorrectionLevel: 'H'  // High redundancy for damaged codes
});
```

### 3. Fallback Mechanisms

Provide alternatives if scanning fails:

```typescript
// Include short URL or manual code
const shortCode = generateShortCode(staticToken);
console.log(`QR Code URL: ${url}`);
console.log(`Manual Code: ${shortCode}`);
// User can type: PORTALR5X9
```

### 4. Testing

Test all physical touchpoints:

```bash
# Generate test QR code
node scripts/generate-qr.js table-test

# Scan with multiple devices
# - iPhone with Nostr wallet
# - Android with Nostr wallet
# - Verify callback received
```

## Why This Matters

Static tokens enable Portal to bridge the **digital and physical worlds**:

🌐 **Online** → Traditional web/mobile authentication
🏪 **In-Person** → QR codes, NFC, physical authentication

This makes Portal unique: **one protocol, infinite touchpoints**.

Whether someone is browsing your website or sitting at your restaurant, they authenticate the same way with the same identity—their Nostr key.

---

**Next Steps**:
- [Authentication Flow](authentication.md) - Core authentication concepts
- [Cashu Tokens](cashu-tokens.md) - Physical tickets with Cashu
- [Single Payments](single-payments.md) - Process payments from physical locations

