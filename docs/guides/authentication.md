# Authentication Flow

Implement secure, passwordless authentication using Nostr and Portal.

## Overview

Portal's authentication is based on Nostr's cryptographic key pairs. Instead of usernames and passwords, users prove their identity by signing challenges with their private keys.

## How It Works

1. **Generate Auth URL**: Your app creates an authentication URL
2. **User Opens URL**: User clicks the link (opens in their Nostr wallet)
3. **Wallet Prompts**: Wallet asks user to approve the authentication
4. **Key Handshake**: Wallet sends user's public key and preferred relays
5. **Challenge-Response**: Your app sends a challenge, user signs it
6. **Verification**: You verify the signature and authenticate the user

## Basic Implementation

### Step 1: Generate Authentication URL

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

const authUrl = await client.newKeyHandshakeUrl((mainKey, preferredRelays) => {
  console.log('User public key:', mainKey);
  console.log('User relays:', preferredRelays);
  
  // Store this information
  // Continue with authentication challenge...
});

console.log('Share this URL with user:', authUrl);
// Example: nostr:nprofile1...
```

### Step 2: Present URL to User

The URL can be shared in multiple ways:

**QR Code:**
```typescript
import QRCode from 'qrcode';

const authUrl = await client.newKeyHandshakeUrl(handleAuth);

// Generate QR code
const qrCodeDataUrl = await QRCode.toDataURL(authUrl);

// Display in HTML
// <img src="${qrCodeDataUrl}" alt="Scan to authenticate" />
```

**Direct Link:**
```html
<a href="${authUrl}">Click to authenticate with your Nostr wallet</a>
```

**Deep Link (Mobile):**
```typescript
// Opens directly in compatible wallets
window.location.href = authUrl;
```

### Step 3: Handle Key Handshake

```typescript
const authUrl = await client.newKeyHandshakeUrl(async (mainKey, preferredRelays) => {
  console.log('Received key handshake from:', mainKey);
  
  // Check if user exists in your database
  const user = await findUserByPubkey(mainKey);
  
  if (!user) {
    console.log('New user, creating account...');
    await createUser(mainKey, preferredRelays);
  }
  
  // Proceed with authentication challenge
  await authenticateUser(mainKey);
});
```

### Step 4: Authenticate the Key

```typescript
async function authenticateUser(mainKey: string) {
  try {
    const authResponse = await client.authenticateKey(mainKey, []);
    
    if (authResponse.status.status === 'approved') {
      console.log('✅ User approved authentication!');
      console.log('Challenge:', authResponse.challenge);
      console.log('User key:', authResponse.user_key);
      
      // Get session token from auth response (issued by user's wallet)
      const sessionToken = authResponse.status.session_token;
      
      // Store session
      await storeSession(mainKey, sessionToken);
      
      return sessionToken;
      
    } else if (authResponse.status.status === 'declined') {
      console.log('❌ User declined authentication');
      console.log('Reason:', authResponse.status.reason);
      return null;
    }
    
  } catch (error) {
    console.error('Authentication error:', error);
    return null;
  }
}
```

## Complete Authentication Example

Here's a complete Express.js example:

```typescript
import express from 'express';
import { PortalSDK } from 'portal-sdk';
import session from 'express-session';

const app = express();

// Session storage
const sessions = new Map<string, { pubkey: string, token: string }>();

// Initialize Portal
const portalClient = new PortalSDK({
  serverUrl: process.env.PORTAL_WS_URL!
});

portalClient.connect().then(() => {
  return portalClient.authenticate(process.env.PORTAL_AUTH_TOKEN!);
});

// Endpoint: Generate authentication URL
app.get('/api/auth/start', async (req, res) => {
  try {
    const authUrl = await portalClient.newKeyHandshakeUrl(
      async (mainKey, preferredRelays) => {
        console.log('Key handshake from:', mainKey);
        
        // Authenticate the user
        const authResponse = await portalClient.authenticateKey(mainKey);
        
        if (authResponse.status.status === 'approved') {
          // Get session token from auth response (issued by user's wallet)
          const sessionToken = authResponse.status.session_token!;
          
          // Store session
          sessions.set(sessionToken, {
            pubkey: mainKey,
            token: sessionToken
          });
          
          console.log('User authenticated:', mainKey);
          
          // In a real app, you might want to notify the frontend
          // via WebSocket or have them poll for status
        }
      }
    );
    
    res.json({ authUrl });
    
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Endpoint: Check authentication status
app.get('/api/auth/status/:pubkey', async (req, res) => {
  const { pubkey } = req.params;
  
  // Find session by pubkey
  const session = Array.from(sessions.values())
    .find(s => s.pubkey === pubkey);
  
  if (session) {
    res.json({
      authenticated: true,
      sessionToken: session.token
    });
  } else {
    res.json({
      authenticated: false
    });
  }
});

// Protected endpoint example
app.get('/api/user/profile', async (req, res) => {
  const authHeader = req.headers.authorization;
  
  if (!authHeader) {
    return res.status(401).json({ error: 'No authorization header' });
  }
  
  const token = authHeader.replace('Bearer ', '');
  const session = sessions.get(token);
  
  if (!session) {
    return res.status(401).json({ error: 'Invalid session token' });
  }
  
  // Fetch user profile from Nostr
  const profile = await portalClient.fetchProfile(session.pubkey);
  
  res.json({
    pubkey: session.pubkey,
    profile
  });
});

function generateRandomToken(): string {
  return Math.random().toString(36).substring(2, 15) + 
         Math.random().toString(36).substring(2, 15);
}

app.listen(3001, () => {
  console.log('Server running on port 3001');
});
```

## Frontend Integration

### React Example

```tsx
import React, { useState, useEffect } from 'react';
import QRCode from 'qrcode';

function LoginPage() {
  const [authUrl, setAuthUrl] = useState<string | null>(null);
  const [qrCode, setQrCode] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    // Generate auth URL when component mounts
    fetch('/api/auth/start')
      .then(res => res.json())
      .then(async data => {
        setAuthUrl(data.authUrl);
        
        // Generate QR code
        const qr = await QRCode.toDataURL(data.authUrl);
        setQrCode(qr);
        
        // Start checking for authentication
        startAuthCheck(data.authUrl);
      });
  }, []);

  function startAuthCheck(url: string) {
    // Extract pubkey from URL (simplified)
    const checkStatus = setInterval(async () => {
      // In reality, you'd extract the pubkey from the auth flow
      // This is simplified for demonstration
      const res = await fetch('/api/auth/status/check');
      const data = await res.json();
      
      if (data.authenticated) {
        clearInterval(checkStatus);
        localStorage.setItem('sessionToken', data.sessionToken);
        window.location.href = '/dashboard';
      }
    }, 2000);
  }

  return (
    <div className="login-page">
      <h1>Login with Nostr</h1>
      
      {qrCode && (
        <div className="qr-code">
          <img src={qrCode} alt="Scan to login" />
          <p>Scan with your Nostr wallet</p>
        </div>
      )}
      
      {authUrl && (
        <div className="direct-link">
          <p>Or click here:</p>
          <a href={authUrl} className="auth-button">
            Open in Nostr Wallet
          </a>
        </div>
      )}
      
      <div className="loading">
        <p>Waiting for authentication...</p>
      </div>
    </div>
  );
}
```

## Advanced: Using Subkeys

Subkeys allow delegated authentication where a user can grant limited permissions to subkeys:

```typescript
const mainKey = 'user-main-public-key';
const subkeys = ['delegated-subkey-1', 'delegated-subkey-2'];

const authResponse = await client.authenticateKey(mainKey, subkeys);

if (authResponse.status.status === 'approved') {
  console.log('Granted permissions:', authResponse.status.granted_permissions);
  console.log('Session token:', authResponse.status.session_token);
}
```

## Static Tokens (Long-lived Auth)

For long-lived authentication URLs that don't expire:

```typescript
const staticToken = 'my-static-token-for-this-integration';

const authUrl = await client.newKeyHandshakeUrl(
  (mainKey) => {
    console.log('User authenticated:', mainKey);
  },
  staticToken  // Static token parameter
);

// This URL can be reused multiple times
console.log('Reusable auth URL:', authUrl);
```

## No-Request Mode

Skip the authentication challenge (just get the key handshake):

```typescript
const authUrl = await client.newKeyHandshakeUrl(
  (mainKey, relays) => {
    // Just store the key, no auth challenge
    console.log('Received key:', mainKey);
  },
  null,  // No static token
  true   // noRequest = true
);
```

## Security Best Practices

### 1. Always Verify Signatures

The Portal SDK handles signature verification, but always check the response status:

```typescript
const authResponse = await client.authenticateKey(mainKey);

if (authResponse.status.status === 'approved') {
  // Safe to proceed
} else {
  // Don't grant access
}
```

### 2. Use Session Tokens

After authentication, issue session tokens instead of storing pubkeys directly:

```typescript
// ✅ Good
const sessionToken = generateSecureToken();
sessions.set(sessionToken, { pubkey: mainKey, expiresAt: Date.now() + 86400000 });
return sessionToken;

// ❌ Bad
// Storing pubkey as session identifier
```

### 3. Implement Session Expiration

```typescript
function validateSession(token: string): boolean {
  const session = sessions.get(token);
  
  if (!session) return false;
  
  if (session.expiresAt < Date.now()) {
    sessions.delete(token);
    return false;
  }
  
  return true;
}
```

### 4. Rate Limiting

Prevent abuse by rate-limiting auth URL generation:

```typescript
const authAttempts = new Map<string, number>();

app.get('/api/auth/start', async (req, res) => {
  const ip = req.ip;
  const attempts = authAttempts.get(ip) || 0;
  
  if (attempts > 10) {
    return res.status(429).json({ error: 'Too many requests' });
  }
  
  authAttempts.set(ip, attempts + 1);
  
  // Generate auth URL...
});
```

### 5. HTTPS Only in Production

```typescript
// Enforce HTTPS in production
if (process.env.NODE_ENV === 'production' && req.protocol !== 'https') {
  return res.redirect('https://' + req.hostname + req.url);
}
```

## Troubleshooting

### User Can't Open Auth URL

**Problem**: URL doesn't open in wallet

**Solutions**:
- Ensure user has a NWC-compatible wallet installed
- Try QR code instead of direct link
- Check URL format is correct (starts with `nostr:`)

### Authentication Never Completes

**Problem**: Callback never fires

**Solutions**:
- Check Portal daemon is connected to relays
- Verify user's wallet is online
- Check firewall/network settings
- Increase timeout if needed

### "Declined" Status

**Problem**: User declined authentication

**Solutions**:
- Show clear explanation of what they're approving
- Allow user to retry
- Log the decline reason for debugging

---

**Next Steps**:
- [Single Payments](single-payments.md) - Accept Lightning payments
- [Profile Management](profiles.md) - Work with user profiles
- [JWT Tokens](jwt-tokens.md) - Session management

