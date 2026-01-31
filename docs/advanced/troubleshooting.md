# Troubleshooting

Common issues and solutions when working with Portal.

## Connection Issues

### "Connection refused" or "ECONNREFUSED"

**Cause**: Portal daemon is not running or not accessible.

**Solutions**:
```bash
# Check if Portal is running
docker ps | grep portal

# Check if port 3000 is listening
netstat -tlnp | grep 3000
# or
lsof -i :3000

# Test connection
curl http://localhost:3000/health

# Check Docker logs
docker logs portal-sdk-daemon
```

### "Connection timeout"

**Cause**: Network issues or firewall blocking connection.

**Solutions**:
- Check firewall rules
- Verify correct URL (ws:// vs wss://)
- Increase timeout in SDK config
- Check network connectivity

```typescript
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws',
  connectTimeout: 30000  // Increase to 30 seconds
});
```

## Authentication Issues

### "Authentication failed"

**Cause**: Invalid or mismatched auth token.

**Solutions**:
```bash
# Verify token in Docker container
docker exec portal-sdk-daemon env | grep PORTAL__AUTH__AUTH_TOKEN

# Verify token in your code
console.log('Using token:', process.env.PORTAL_AUTH_TOKEN?.substring(0, 10) + '...');

# Regenerate token if needed
NEW_TOKEN=$(openssl rand -hex 32)
echo "New token: $NEW_TOKEN"
```

### User Can't Authenticate

**Cause**: User doesn't have compatible wallet or URL doesn't open.

**Solutions**:
- Verify user has Alby, Mutiny, or compatible NWC wallet
- Try QR code instead of direct link
- Check relay connectivity
- Verify PORTAL__NOSTR__PRIVATE_KEY is set correctly

```bash
# Test relay connectivity
wscat -c wss://relay.damus.io

# Verify key format (64 hex chars)
echo $PORTAL__NOSTR__PRIVATE_KEY | wc -c  # Should output 65 (64 + newline)
```

## Payment Issues

### Payments Never Complete

**Cause**: Multiple possible reasons.

**Solutions**:
```typescript
// Add timeout handling
const TIMEOUT = 120000; // 2 minutes
const timeout = setTimeout(() => {
  console.log('Payment timed out');
  // Handle timeout
}, TIMEOUT);

client.requestSinglePayment(user, [], request, (status) => {
  if (status.status === 'paid') {
    clearTimeout(timeout);
    // Success
  }
});
```

### "User rejected" or "User failed"

**Cause**: User declined or payment failed.

**Common reasons**:
- Insufficient funds
- Lightning routing failure
- User manually declined
- Channel capacity issues

**Solutions**:
- Show clear payment details upfront
- Ensure reasonable amounts
- Provide fallback payment options
- Check NWC wallet has sufficient balance

### NWC Not Working

**Cause**: Invalid or expired NWC URL.

**Solutions**:
```bash
# Verify NWC URL format (PORTAL__WALLET__NWC__URL)
echo $PORTAL__WALLET__NWC__URL
# Should start with: nostr+walletconnect://

# Test NWC connection separately
# Use a tool like Alby to verify NWC string works

# Regenerate NWC URL in wallet settings
```

## Relay Issues

### "Cannot connect to relays"

**Cause**: Relay URLs invalid or relays offline.

**Solutions**:
```bash
# Test relay connectivity
for relay in wss://relay.damus.io wss://relay.snort.social wss://nos.lol; do
  echo "Testing $relay"
  timeout 5 wscat -c $relay && echo "✅ Connected" || echo "❌ Failed"
done

# Update PORTAL__NOSTR__RELAYS in .env
PORTAL__NOSTR__RELAYS=wss://relay.damus.io,wss://relay.snort.social,wss://nos.lol
```

### Messages Not Delivering

**Cause**: Not enough relays or user not on same relays.

**Solutions**:
- Use user's preferred relays from handshake
- Connect to 3-5 popular relays
- Add paid relays for better reliability

```typescript
// Add user's preferred relays
client.newKeyHandshakeUrl(async (mainKey, preferredRelays) => {
  // Add user's relays
  for (const relay of preferredRelays) {
    try {
      await client.addRelay(relay);
    } catch (e) {
      console.error('Failed to add relay:', relay);
    }
  }
});
```

## Docker Issues

### Container Won't Start

```bash
# Check logs
docker logs portal-sdk-daemon

# Check environment variables
docker inspect portal-sdk-daemon | grep -A 20 Env

# Verify Docker image
docker images | grep portal

# Remove and recreate
docker rm -f portal-sdk-daemon
docker run -d --name portal-sdk-daemon \
  -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=$PORTAL__AUTH__AUTH_TOKEN \
  -e PORTAL__NOSTR__PRIVATE_KEY=$PORTAL__NOSTR__PRIVATE_KEY \
  getportal/sdk-daemon:latest
```

### Health Check Failing

```bash
# Manual health check
curl http://localhost:3000/health

# Check if service is listening
docker exec portal-sdk-daemon netstat -tlnp

# Check for errors in logs
docker logs portal-sdk-daemon --tail 50
```

## JavaScript SDK issues

### "Cannot find module 'portal-sdk'"

```bash
# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Verify installation
npm list portal-sdk

# Check import path
# ✅ Correct
import { PortalSDK } from 'portal-sdk';

# ❌ Incorrect
import { PortalSDK } from './portal-sdk';
```

### WebSocket Errors in Browser

```javascript
// Check if using correct protocol
const url = window.location.protocol === 'https:' 
  ? 'wss://portal.example.com/ws'
  : 'ws://localhost:3000/ws';

const client = new PortalSDK({ serverUrl: url });
```

### TypeScript / module errors

```bash
# Ensure TypeScript is installed
npm install --save-dev typescript

# Check tsconfig.json settings
{
  "compilerOptions": {
    "esModuleInterop": true,
    "skipLibCheck": true
  }
}
```

## Performance Issues

### Slow Response Times

**Causes**:
- Too many relays
- Slow relay connections
- Network latency

**Solutions**:
- Reduce to 3-5 fast relays
- Use geographically close relays
- Monitor relay performance

### High Memory Usage

```bash
# Check Docker stats
docker stats portal-sdk-daemon

# Restart container
docker restart portal-sdk-daemon

# Adjust Docker memory limits
docker run -d --name portal \
  --memory=512m \
  --memory-swap=1g \
  ...
```

## Debug Mode

Enable verbose logging for troubleshooting:

```typescript
// In your SDK code
const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

// Log all messages
client.on({
  onConnected: () => console.log('[DEBUG] Connected'),
  onDisconnected: () => console.log('[DEBUG] Disconnected'),
  onError: (e) => console.error('[DEBUG] Error:', e)
});
```

For Docker daemon:

```bash
# Set log level
docker run -d \
  -e RUST_LOG=debug \
  ...
  getportal/sdk-daemon:latest

# View debug logs
docker logs -f portal-sdk-daemon
```

## Getting Help

If you're still having issues:

1. **Check existing issues**: [GitHub Issues](https://github.com/PortalTechnologiesInc/lib/issues)
2. **Search documentation**: Use Ctrl+F or search feature
3. **Enable debug logging**: Capture detailed logs
4. **Create minimal reproduction**: Simplify to smallest failing example
5. **Open an issue**: Include:
   - Portal version
   - SDK version
   - Environment (OS, Node version)
   - Complete error messages
   - Steps to reproduce

---

**Back to**: [Documentation Home](../README.md)

