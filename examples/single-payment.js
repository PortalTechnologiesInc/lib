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

  // Listen for intermediate events (e.g. user_approved)
  client.onEvent(op.streamId, (event) => {
    console.log('Event:', event);
  });

  // Poll for the terminal event (paid / rejected / timeout / error)
  const result = await client.poll(op);

  const status = result.status;
  const reason = result.reason ?? '';
  const preimage = result.preimage ?? '';


  if (status === 'paid') {
    console.log('✓ Payment received!');
    console.log('  Preimage:', preimage);
  } else if (status === 'user_rejected') {
    console.log('✗ User rejected the payment. Reason:', reason);
  } else if (status === 'timeout') {
    console.log('✗ Payment timed out.');
  } else {
    console.log('✗ Payment failed:', status, reason);
  }
}

main().catch((err) => {
  console.error('Error:', err.message ?? err);
  process.exit(1);
});
