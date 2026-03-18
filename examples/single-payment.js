/**
 * Single payment example
 *
 * Requests a one-time Lightning payment from a user and polls for the result.
 *
 * Usage:
 *   PORTAL_URL=http://localhost:3000 PORTAL_TOKEN=your-token MAIN_KEY=user-pubkey-hex node single-payment.js
 */

import 'dotenv/config';
import { PortalClient, Currency } from 'portal-sdk';

const BASE_URL = process.env.PORTAL_URL ?? 'http://localhost:3000';
const AUTH_TOKEN = process.env.PORTAL_TOKEN ?? 'your-secret-token';
const MAIN_KEY = process.env.MAIN_KEY ?? 'replace-with-user-pubkey-hex';

const client = new PortalClient({ baseUrl: BASE_URL, authToken: AUTH_TOKEN });

async function main() {
  console.log(`Requesting payment from ${MAIN_KEY}...`);

  const op = await client.requestSinglePayment(
    MAIN_KEY,
    [], // subkeys
    {
      description: 'Test payment — 10 sats',
      amount: 10_000, // millisats (10 sats)
      currency: Currency.Millisats,
    }
  );

  console.log('Stream ID:', op.streamId);
  console.log('Waiting for user to approve in their wallet...');

  // Poll for the terminal event (paid / rejected / timeout / error)
  const result = await client.poll(op);

  if (result.status === 'paid') {
    console.log('✓ Payment received!');
    console.log('  Preimage:', result.preimage);
  } else if (result.status === 'user_rejected') {
    console.log('✗ User rejected the payment.');
  } else if (result.status === 'timeout') {
    console.log('✗ Payment timed out.');
  } else {
    console.log('✗ Payment failed:', result.status, result.reason ?? '');
  }
}

main().catch((err) => {
  console.error('Error:', err.message ?? err);
  process.exit(1);
});
