/**
 * Authentication flow example
 *
 * 1. Generates a key handshake URL
 * 2. Waits for the user to scan it with a Nostr wallet
 * 3. Authenticates the user's key and prints the result
 *
 * Usage:
 *   PORTAL_URL=http://localhost:3000 PORTAL_TOKEN=your-token node auth.js
 */

import 'dotenv/config';
import { PortalClient } from 'portal-sdk';
import QRCode from 'qrcode';

const BASE_URL = process.env.PORTAL_URL ?? 'http://localhost:3000';
const AUTH_TOKEN = process.env.PORTAL_TOKEN ?? 'your-secret-token';

const client = new PortalClient({ baseUrl: BASE_URL, authToken: AUTH_TOKEN });

async function main() {
  // Step 1: get a handshake URL to show to the user
  console.log('Generating key handshake URL...');
  const handshake = await client.newKeyHandshakeUrl();
  const keyHandshakeUrl = handshake.url;

  console.log('\nKey handshake URL (share with your user):');
  console.log(keyHandshakeUrl);

  console.log('\nQR code (scan this in your Nostr wallet):');
  try {
    const qr = await QRCode.toString(keyHandshakeUrl, { type: 'terminal', small: true });
    console.log(qr);
  } catch (err) {
    // If terminal QR rendering fails for any reason, at least the URL is still available.
    console.error('Could not render QR code in terminal:', err?.message ?? err);
  }
  console.log('\nWaiting for user to scan...');

  // Step 2: poll until the user completes the handshake
  const handshakeResult = await client.poll(handshake);
  const mainKey = handshakeResult.main_key;
  console.log('\n✓ User connected! Public key:', mainKey);

  // Step 3: authenticate the key
  console.log('\nAuthenticating key...');
  const authOp = await client.authenticateKey(mainKey);
  const authResult = await client.poll(authOp);

  if (authResult.status.status === 'approved') {
    console.log('✓ Authentication approved!');
    if (authResult.status.session_token) {
      console.log('  Session token:', authResult.status.session_token);
    }
  } else {
    console.log('✗ Authentication rejected:', authResult.status.reason ?? 'no reason given');
  }
}

main().catch((err) => {
  console.error('Error:', err.message ?? err);
  process.exit(1);
});
