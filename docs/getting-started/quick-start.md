# Quick Start

Get Portal up and running in under 5 minutes!

## Overview

This guide will help you:
1. Deploy the Portal SDK Daemon using Docker
2. Install the TypeScript SDK
3. Create your first authentication flow

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed
- [Node.js](https://nodejs.org/) 18+ and npm
- A Nostr private key (we'll show you how to generate one)
- (Optional) A Lightning wallet with Nostr Wallet Connect support

## Step 1: Generate a Nostr Key

Your Portal instance needs a Nostr private key to operate. You can generate one using:

**Option A: Using nostrtool.com (easiest)**
- Visit [nostrtool.com](https://nostrtool.com/)
- Click on "Key Generator"
- Copy your private key (nsec...)
- **Important**: Do this offline or in a private browser window for maximum security

**Option B: Using a Nostr client**
- Download [Alby Extension](https://getalby.com/)
- Create an account
- Go to Settings → Developer Settings → Export Keys
- Copy your private key (nsec...)

**Option C: Using a command-line tool**
```bash
# Install nak (Nostr Army Knife)
npm install -g nak

# Generate a new key pair
nak key generate
```

**Important**: Keep your private key secure. Anyone with access to it can impersonate your Portal instance.

## Step 2: Deploy with Docker

Run the Portal SDK Daemon:

```bash
docker run --rm --name portal-sdk-daemon -d \
  -p 3000:3000 \
  -e AUTH_TOKEN=your-secret-auth-token-change-this \
  -e NOSTR_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Replace:
- `your-secret-auth-token-change-this` with a strong random token
- `your-nostr-private-key-hex` with your Nostr private key in hex format

**Verify it's running:**
```bash
curl http://localhost:3000/health
# Should return: OK
```

## Step 3: Install the TypeScript SDK

In your project directory:

```bash
npm install portal-sdk
```

## Step 4: Your First Integration

Create a file `portal-demo.js`:

```javascript
import { PortalSDK } from 'portal-sdk';

async function main() {
  // Initialize the SDK
  const client = new PortalSDK({
    serverUrl: 'ws://localhost:3000/ws'
  });

  // Connect to the server
  await client.connect();
  console.log('Connected to Portal!');

  // Authenticate with your token
  await client.authenticate('your-secret-auth-token-change-this');
  console.log('Authenticated!');

  // Generate an authentication URL for a user
  const url = await client.newKeyHandshakeUrl((mainKey) => {
    console.log('User authenticated with key:', mainKey);
    
    // Here you would typically:
    // - Create a user account
    // - Generate a session token
    // - Store the user's public key
  });

  console.log('\n🎉 Authentication URL generated!');
  console.log('Share this URL with your user:');
  console.log(url);
  console.log('\nWaiting for user authentication...');
}

main().catch(console.error);
```

Run it:

```bash
node portal-demo.js
```

## Step 5: Test Authentication

1. The script will output an authentication URL
2. Open the URL in a browser (or share it with a user)
3. If you have Alby or another NWC-compatible wallet, it will ask you to approve the connection
4. Once approved, your script will log the user's public key

**Congratulations!** 🎉 You've just authenticated a user without any passwords, email verification, or centralized auth service.

## What's Next?

Now that you have Portal running, explore:

- **[Process Payments](../guides/single-payments.md)**: Accept Lightning payments
- **[Issue Cashu Tokens](../guides/cashu-tokens.md)**: Create tickets and vouchers for users
- **[Run Your Own Mint](../guides/running-a-mint.md)**: Deploy a custom Cashu mint for tickets
- **[JWT Tokens](../guides/jwt-tokens.md)**: Session management and API authentication
- **[Profile Management](../guides/profiles.md)**: Fetch user profiles from Nostr
- **[Production Deployment](../advanced/production.md)**: Deploy Portal for production use

## Common Issues

### "Connection refused"
- Make sure Docker container is running: `docker ps`
- Check the port is correct (default: 3000)

### "Authentication failed"
- Verify your AUTH_TOKEN matches between Docker and SDK
- Check Docker logs: `docker logs portal-sdk-daemon`

### "Invalid Nostr key"
- Ensure your key is in hex format (not nsec)
- Convert nsec to hex using: `nak decode nsec your-key-here`

## Example Projects

Check out complete examples:
- [Authentication Flow](../examples/authentication-flow.md)
- [Payment Integration](../examples/payment-integration.md)
- [Subscription Service](../examples/subscription-service.md)

---

**Need Help?** Check the [FAQ](../resources/faq.md) or [Troubleshooting Guide](../advanced/troubleshooting.md)

