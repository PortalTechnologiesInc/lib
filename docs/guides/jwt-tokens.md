# JWT Tokens (Session Management)

Verify JWT tokens issued by user wallet apps for API authentication.

## Overview

While Cashu tokens are used for tickets and transferable access, JWT tokens are for:
- API authentication (user's wallet issues the token, you verify it)
- Session management
- Short-lived access tokens
- Stateless authentication

**Important**: In most cases, JWT tokens are **issued by the user's wallet app** and **verified by your business**. You don't typically issue JWTs yourself - the user's wallet does this after authentication.

## Primary Use Case: Verifying JWT Tokens

The main use of JWT tokens in Portal is **verification**. After a user authenticates through their wallet app, they receive a JWT token from their wallet. Your business then verifies this token to authenticate API requests.

### Verifying JWT Tokens

```typescript
const publicKey = 'your-service-public-key';
const token = 'jwt-token-from-user';

try {
  const claims = await client.verifyJwt(publicKey, token);
  console.log('Token is valid for user:', claims.target_key);
  
  // Grant access
} catch (error) {
  console.error('Invalid or expired token');
  // Deny access
}
```

## Advanced: Issuing JWT Tokens (Less Common)

In some cases, you may want to issue JWT tokens yourself (e.g., for service-to-service authentication):

```typescript
const targetPubkey = 'user-public-key';
const durationHours = 24; // Token valid for 24 hours

const jwtToken = await client.issueJwt(targetPubkey, durationHours);

console.log('JWT:', jwtToken);
```

However, in most authentication flows, the user's wallet app will issue the JWT token after they approve the authentication request.

## API Authentication Middleware

```typescript
async function authenticateRequest(req, res, next) {
  const authHeader = req.headers.authorization;
  
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'No token provided' });
  }
  
  const token = authHeader.substring(7);
  
  try {
    const claims = await portalClient.verifyJwt(
      process.env.SERVICE_PUBKEY,
      token
    );
    
    req.userPubkey = claims.target_key;
    next();
  } catch (error) {
    return res.status(401).json({ error: 'Invalid token' });
  }
}

// Protected route
app.get('/api/user/data', authenticateRequest, (req, res) => {
  res.json({ userPubkey: req.userPubkey, data: '...' });
});
```

## Typical Authentication Flow with JWTs

1. User initiates authentication through your app
2. User's Nostr wallet app generates and signs a JWT
3. JWT is sent to your application
4. Your application verifies the JWT using Portal
5. Grant API access based on verified identity

## Best Practices

1. **Verify, don't issue**: Let user wallets issue tokens, you just verify them
2. **Check expiration**: Validate JWT expiration times
3. **Secure transmission**: Always use HTTPS
4. **Don't log tokens**: Never log tokens in production
5. **Use for APIs**: Perfect for stateless API authentication

---

**Next**: [Relay Management](relays.md)

